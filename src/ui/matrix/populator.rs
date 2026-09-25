use std::time::{Duration, Instant};

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Cell, Row},
};

use super::color;
use super::Cursor;
use crate::ports::PortTable;

// How long a changed cell's background blends from FLASH toward its normal
// status color. 4x SCAN_INTERVAL (500ms) so a change spans several scan
// cycles before fading out — long enough not to be missed, short enough
// that "recent" keeps meaning in a grid that's always changing somewhere.
const FLASH_DURATION: Duration = Duration::from_millis(2000);

fn flash_weight(last_changed: Option<Instant>) -> f32 {
    match last_changed {
        Some(t) => (1.0 - t.elapsed().as_secs_f32() / FLASH_DURATION.as_secs_f32()).clamp(0.0, 1.0),
        None => 0.0,
    }
}

// Strongest flash weight of any port in `row` — drives the row label's own
// fade, a coarser "something changed in this row" signal alongside the
// precise per-cell flash, useful since a single changed cell can be easy to
// spot-miss among 128 columns.
fn row_flash_weight(row: usize, last_changed: &[Option<Instant>], grid_cols: usize) -> f32 {
    let base = row * grid_cols;
    let end = (base + grid_cols).min(last_changed.len());
    last_changed[base..end]
        .iter()
        .map(|&t| flash_weight(t))
        .fold(0.0, f32::max)
}

// The port space is a grid (row = port / grid_cols, col = port % grid_cols).
// grid_cols grows continuously with terminal width between these bounds
// instead of staying fixed; 64 is the narrowest a `w`-cycled fixed width can
// go (see `WidthMode::FIXED_STEPS`), and 1024 keeps rows from shrinking to
// an unreadably small count.
// grid_cols need not evenly divide PORT_COUNT: grid_rows is a ceiling
// division, so the final row is a partial one, its unused trailing cells
// rendered blank by `fill_row` rather than indexing past the last real port.
pub const MIN_GRID_COLS: usize = 64;
pub const MAX_GRID_COLS: usize = 1024;

// Column count that fills `available_width` (after reserving the label
// column and right margin), clamped to [MIN_GRID_COLS, MAX_GRID_COLS].
pub fn grid_cols(available_width: u16) -> usize {
    let usable = available_width.saturating_sub(LABEL_WIDTH as u16 + 1) as usize;
    usable.clamp(MIN_GRID_COLS, MAX_GRID_COLS)
}

pub fn grid_rows(grid_cols: usize) -> usize {
    crate::ports::PORT_COUNT.div_ceil(grid_cols)
}

// Right-edge sliver for the vertical grid line, paired with a real
// underline (a separate render attribute, not a glyph) for the horizontal
// one — together they outline the cell's own bottom-right border instead of
// cutting across its center like a full-width/height glyph would.
const GRID_GLYPH: &str = "▕";

// Marks the cell under the cursor. A full block, rather than reversing
// GRID_GLYPH's fg/bg (the previous approach), since GRID_GLYPH only inks a
// thin vertical sliver — reversed, that mostly read as a plain black
// square. The cell's actual status color is already named in the selection
// box below, so trading it for an unmissable cursor here is a fair swap.
const CURSOR_GLYPH: &str = "█";

// Width of the row's port-range label column — exactly wide enough for the
// widest label, "65408-65535" (11 characters); left-aligned, so shorter
// labels pad out on the right instead of needing a spacer column.
pub const LABEL_WIDTH: usize = 11;

// Rows covering the well-known ports (0-1023) always render individually
// (never collapsing into a black bar) and stay pinned at the top of the
// viewport, so that range is visible at all times regardless of what's
// active on it or where the cursor scrolls. Ceiling division so a row that
// straddles the 1024 boundary (grid_cols not a divisor of 1024) is still
// pinned in full rather than split.
pub fn pinned_rows(grid_cols: usize) -> usize {
    1024usize.div_ceil(grid_cols)
}

// True when every port in `row` is closed and none is currently flashing.
// Rows within `pinned_rows(grid_cols)` are never considered closed,
// regardless of their actual state. A flashing port (e.g. one that just
// closed) keeps its row visible for the full fade window even though its
// `PortStatus` alone would call the row all-closed — otherwise a change
// could flash and vanish in the same frame its row collapses out of the
// display list.
fn row_is_closed(
    row: usize,
    ports: &PortTable,
    last_changed: &[Option<Instant>],
    grid_cols: usize,
) -> bool {
    if row < pinned_rows(grid_cols) {
        return false;
    }
    let base = row * grid_cols;
    let end = (base + grid_cols).min(crate::ports::PORT_COUNT);
    (base..end).all(|port| {
        !ports[port].tcp_listen
            && !ports[port].tcp_established
            && !ports[port].udp_active
            && flash_weight(last_changed[port]) <= 0.0
    })
}

// Real grid rows with anything to show, in ascending order — every row
// within `pinned_rows(grid_cols)`, plus every other row that isn't
// all-closed. Runs of all-closed rows outside the pinned head are dropped
// entirely rather than being rendered as rows of their own; `Cursor::row`/
// scrolling operate on this list, so the number of display rows shrinks and
// grows as port state changes.
fn build_display_rows(
    ports: &PortTable,
    last_changed: &[Option<Instant>],
    grid_cols: usize,
) -> Vec<usize> {
    (0..grid_rows(grid_cols))
        .filter(|&row| !row_is_closed(row, ports, last_changed, grid_cols))
        .collect()
}

// Number of rows the grid renders as, after hiding closed runs — always <=
// grid_rows(grid_cols), and what `Cursor::row`/scrolling should treat as the
// row count instead of the raw row count.
pub fn display_row_count(
    ports: &PortTable,
    last_changed: &[Option<Instant>],
    grid_cols: usize,
) -> usize {
    build_display_rows(ports, last_changed, grid_cols).len()
}

// Raw port number under the cursor, reversing the same display-row mapping
// `ports_matrix` renders with. Clamped to `PORT_COUNT - 1`: the final grid
// row can be partial (grid_cols need not divide PORT_COUNT), so a cursor
// sitting past its last real column would otherwise land past the end of
// the port table.
pub fn selected_port(
    ports: &PortTable,
    last_changed: &[Option<Instant>],
    cursor: Cursor,
    grid_cols: usize,
) -> usize {
    let display_rows = build_display_rows(ports, last_changed, grid_cols);
    let idx = cursor.row.min(display_rows.len().saturating_sub(1));
    (display_rows[idx] * grid_cols + cursor.col).min(crate::ports::PORT_COUNT - 1)
}

// Renders `visible_rows` display rows total, made of two parts: the pinned
// head (display rows `0..pinned_rows(grid_cols)`, always shown, never
// scrolled) and a scrolling body starting at `pinned_rows(grid_cols) +
// body_offset` filling whatever height remains — the viewport `layout::tui`
// positions to keep the cursor in view within that remaining space, rather
// than the full display list at once.
pub fn ports_matrix(
    ports: &PortTable,
    cursor: Cursor,
    body_offset: usize,
    visible_rows: usize,
    last_changed: &[Option<Instant>],
    grid_cols: usize,
) -> Vec<Row<'static>> {
    let display_rows = build_display_rows(ports, last_changed, grid_cols);
    let pinned = pinned_rows(grid_cols)
        .min(visible_rows)
        .min(display_rows.len());

    let mut rows = Vec::with_capacity(visible_rows);
    for (display_idx, &row) in display_rows[..pinned].iter().enumerate() {
        rows.push(fill_row(
            row,
            ports,
            cursor,
            display_idx,
            last_changed,
            grid_cols,
        ));
    }

    let body_budget = visible_rows - pinned;
    let body_start = (pinned + body_offset).min(display_rows.len());
    let body_end = (body_start + body_budget).min(display_rows.len());
    for (i, &row) in display_rows[body_start..body_end].iter().enumerate() {
        rows.push(fill_row(
            row,
            ports,
            cursor,
            body_start + i,
            last_changed,
            grid_cols,
        ));
    }

    rows
}

fn fill_row(
    row: usize,
    ports: &PortTable,
    cursor: Cursor,
    display_idx: usize,
    last_changed: &[Option<Instant>],
    grid_cols: usize,
) -> Row<'static> {
    let mut cells: Vec<Cell> = Vec::with_capacity(grid_cols + 4);

    // Left margin, paired with the right one below, to center the grid
    // horizontally in the table area.
    cells.push(Cell::new(""));

    let range_start = row * grid_cols;
    let range_end = (range_start + grid_cols - 1).min(crate::ports::PORT_COUNT - 1);
    let label = Line::from(format!("{range_start}-{range_end}")).alignment(Alignment::Right);
    let label_weight = row_flash_weight(row, last_changed, grid_cols);
    let label_fg = if label_weight > 0.0 {
        color::lerp_color(color::LABEL, color::FLASH, label_weight)
    } else {
        color::LABEL
    };
    cells.push(Cell::from(label).style(Style::default().fg(label_fg)));

    // One-column gap between the label and the grid itself.
    cells.push(Cell::new(""));

    for content_col in 0..grid_cols {
        let port = row * grid_cols + content_col;

        // The final grid row can be partial (grid_cols need not divide
        // PORT_COUNT) — trailing columns past the last real port render as
        // plain blanks rather than indexing off the end of the port table.
        let cell = if port >= crate::ports::PORT_COUNT {
            Cell::new("")
        } else if display_idx == cursor.row && content_col == cursor.col {
            Cell::from(CURSOR_GLYPH).style(Style::default().fg(color::CURSOR))
        } else {
            let status = &ports[port];
            let base_bg = color::status_color(status, port < 1024);
            let weight = flash_weight(last_changed[port]);
            let bg = if weight > 0.0 {
                color::lerp_color(base_bg, color::FLASH, weight)
            } else {
                base_bg
            };
            let border = color::process_border_color(status.process.as_deref());
            let bind_global = status.bind_global && (status.tcp_listen || status.udp_active);
            Cell::from(GRID_GLYPH).style(grid_cell_style(bg, border, bind_global))
        };
        cells.push(cell);
    }

    // Right margin, to keep the grid off the table's border.
    cells.push(Cell::new(""));

    Row::new(cells)
}

// Right-edge glyph + bottom underline, over the cell's own status
// background (blended toward FLASH while recently changed). `border` names
// the owning process via a stable hue (or falls back to plain black for
// closed/unresolved cells); `bind_global` renders that border bold, as an
// independent visual weight layered on top, so bind-scope reads as its own
// signal rather than competing for the same color channel. Underline color
// is left to the terminal default (which follows `fg`) rather than set
// explicitly — the explicit-underline-color SGR is a newer extension that
// some terminals (e.g. Terminal.app) mishandle, which was dropping the line
// entirely.
fn grid_cell_style(bg: Color, border: Color, bind_global: bool) -> Style {
    let mut style = Style::default()
        .bg(bg)
        .fg(border)
        .add_modifier(Modifier::UNDERLINED);
    if bind_global {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}
