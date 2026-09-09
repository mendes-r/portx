#[path = "./color.rs"]
mod color;

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Cell, Row},
};

use super::Cursor;
use crate::ports::PortTable;

// The port space is a 128x512 grid (row = port / 128, col = port % 128,
// 128 * 512 = 65536). 128 columns fits comfortably in an ordinary terminal
// window, unlike a square 256x256 layout would — the height is handled by
// scrolling a viewport of rows instead, so every rendered cell still maps
// to exactly one real port.
pub const GRID_COLS: usize = 128;
pub const GRID_ROWS: usize = 512;

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
// widest label, "65408-65535" (11 characters); right-aligned, so shorter
// labels pad out on the left instead of needing a spacer column.
pub const LABEL_WIDTH: usize = 11;

// Rows covering the well-known ports (0-1023, i.e. rows 0-7 at 128 columns
// per row) always render individually (never collapsing into a black bar)
// and stay pinned at the top of the viewport, so that range is visible at
// all times regardless of what's active on it or where the cursor scrolls.
pub const PINNED_ROWS: usize = 1024 / GRID_COLS;

// True when every port in `row` is closed. Rows within `PINNED_ROWS` are
// never considered closed, regardless of their actual state.
fn row_is_closed(row: usize, ports: &PortTable) -> bool {
    if row < PINNED_ROWS {
        return false;
    }
    let base = row * GRID_COLS;
    (base..base + GRID_COLS).all(|port| {
        !ports[port].tcp_listen && !ports[port].tcp_established && !ports[port].udp_active
    })
}

// Real grid rows with anything to show, in ascending order — every row
// within `PINNED_ROWS`, plus every other row that isn't all-closed. Runs of
// all-closed rows outside `PINNED_ROWS` are dropped entirely rather than
// being rendered as rows of their own; `Cursor::row`/scrolling operate on
// this list, so the number of display rows shrinks and grows as port state
// changes.
fn build_display_rows(ports: &PortTable) -> Vec<usize> {
    (0..GRID_ROWS)
        .filter(|&row| !row_is_closed(row, ports))
        .collect()
}

// Number of rows the grid renders as, after hiding closed runs — always <=
// GRID_ROWS, and what `Cursor::row`/scrolling should treat as the row count
// instead of the raw GRID_ROWS constant.
pub fn display_row_count(ports: &PortTable) -> usize {
    build_display_rows(ports).len()
}

// Everything the selection box needs to describe the cell under the
// cursor: the header line (which port) and the same label/color the cell
// itself is drawn with.
pub struct CellInfo {
    pub header: String,
    pub label: String,
    pub color: Color,
    // Process backing the cell's state, e.g. "nginx (1234)"; absent for
    // closed ports, or when it couldn't be resolved.
    pub process: Option<String>,
}

// Raw port number under the cursor, reversing the same display-row mapping
// `ports_matrix` renders with.
pub fn selected_port(ports: &PortTable, cursor: Cursor) -> usize {
    let display_rows = build_display_rows(ports);
    let idx = cursor.row.min(display_rows.len().saturating_sub(1));
    display_rows[idx] * GRID_COLS + cursor.col
}

pub fn selected_cell_info(ports: &PortTable, cursor: Cursor) -> CellInfo {
    let port = selected_port(ports, cursor);
    CellInfo {
        header: format!("port {port}"),
        label: color::status_label(&ports[port]).to_string(),
        color: color::status_color(&ports[port], port < 1024),
        process: ports[port].process.clone(),
    }
}

// Renders `visible_rows` display rows total, made of two parts: the pinned
// head (display rows `0..PINNED_ROWS`, always shown, never scrolled) and a
// scrolling body starting at `PINNED_ROWS + body_offset` filling whatever
// height remains — the viewport `layout::tui` positions to keep the cursor
// in view within that remaining space, rather than the full display list at
// once.
pub fn ports_matrix(
    ports: &PortTable,
    cursor: Cursor,
    body_offset: usize,
    visible_rows: usize,
) -> Vec<Row<'static>> {
    let display_rows = build_display_rows(ports);
    let pinned = PINNED_ROWS.min(visible_rows).min(display_rows.len());

    let mut rows = Vec::with_capacity(visible_rows);
    for (display_idx, &row) in display_rows[..pinned].iter().enumerate() {
        rows.push(fill_row(row, ports, cursor, display_idx));
    }

    let body_budget = visible_rows - pinned;
    let body_start = (pinned + body_offset).min(display_rows.len());
    let body_end = (body_start + body_budget).min(display_rows.len());
    for (i, &row) in display_rows[body_start..body_end].iter().enumerate() {
        rows.push(fill_row(row, ports, cursor, body_start + i));
    }

    rows
}

fn fill_row(row: usize, ports: &PortTable, cursor: Cursor, display_idx: usize) -> Row<'static> {
    let mut cells: Vec<Cell> = Vec::with_capacity(GRID_COLS + 2);

    let range_start = row * GRID_COLS;
    let range_end = range_start + GRID_COLS - 1;
    let label = Line::from(format!("{range_start}-{range_end}")).alignment(Alignment::Right);
    cells.push(Cell::from(label).style(Style::default().fg(Color::DarkGray)));

    for content_col in 0..GRID_COLS {
        let port = row * GRID_COLS + content_col;

        let cell = if display_idx == cursor.row && content_col == cursor.col {
            Cell::from(CURSOR_GLYPH).style(Style::default().fg(color::CURSOR))
        } else {
            let bg = color::status_color(&ports[port], port < 1024);
            Cell::from(GRID_GLYPH).style(grid_cell_style(bg))
        };
        cells.push(cell);
    }

    // Right margin, to keep the grid off the table's border.
    cells.push(Cell::new(""));

    Row::new(cells)
}

// Right-edge glyph + bottom underline, both reading as the grid line color,
// over the cell's own status background. Underline color is left to the
// terminal default (which follows `fg`) rather than set explicitly — the
// explicit-underline-color SGR is a newer extension that some terminals
// (e.g. Terminal.app) mishandle, which was dropping the line entirely.
fn grid_cell_style(bg: Color) -> Style {
    Style::default()
        .bg(bg)
        .fg(color::GRID_LINE)
        .add_modifier(Modifier::UNDERLINED)
}
