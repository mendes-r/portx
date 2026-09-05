#[path = "./color.rs"]
mod color;

use ratatui::{
    style::{Color, Modifier, Style},
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

// SSH, called out in red regardless of its actual status.
const HIGHLIGHTED_PORT: usize = 22;

// Right-edge sliver for the vertical grid line, paired with a real
// underline (a separate render attribute, not a glyph) for the horizontal
// one — together they outline the cell's own bottom-right border instead of
// cutting across its center like a full-width/height glyph would.
const GRID_GLYPH: &str = "▕";

// Rows covering the well-known ports (0-1023, i.e. rows 0-7 at 128 columns
// per row) always render individually (never collapsing into a black bar)
// and stay pinned at the top of the viewport, so that range is visible at
// all times regardless of what's active on it or where the cursor scrolls.
pub const PINNED_ROWS: usize = 1024 / GRID_COLS;

// True when every port in `row` is closed. The row containing the
// highlighted port is never considered closed, even if that port itself is
// inactive — it always renders in red, so it can't be hidden along with the
// genuinely empty rows around it. Same for rows within `PINNED_ROWS`.
fn row_is_closed(row: usize, ports: &PortTable) -> bool {
    if row < PINNED_ROWS {
        return false;
    }
    let base = row * GRID_COLS;
    (base..base + GRID_COLS).all(|port| {
        port != HIGHLIGHTED_PORT
            && !ports[port].tcp_listen
            && !ports[port].tcp_established
            && !ports[port].udp_active
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

pub fn selected_cell_info(ports: &PortTable, cursor: Cursor) -> CellInfo {
    let display_rows = build_display_rows(ports);
    let idx = cursor.row.min(display_rows.len().saturating_sub(1));
    let row = display_rows[idx];

    let port = row * GRID_COLS + cursor.col;
    let (label, color) = if port == HIGHLIGHTED_PORT {
        ("ssh (highlighted)".to_string(), color::HIGHLIGHTED_PORT)
    } else {
        (
            color::status_label(&ports[port]).to_string(),
            color::status_color(&ports[port]),
        )
    };
    CellInfo {
        header: format!("port {port}"),
        label,
        color,
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

    for col in 0..GRID_COLS + 2 {
        if col == 0 || col == GRID_COLS + 1 {
            // Margins for centering real matrix content
            cells.push(Cell::new(""));
            continue;
        }

        let content_col = col - 1;
        let port = row * GRID_COLS + content_col;
        let bg = if port == HIGHLIGHTED_PORT {
            color::HIGHLIGHTED_PORT
        } else {
            color::status_color(&ports[port])
        };

        let mut style = grid_cell_style(bg);
        if display_idx == cursor.row && content_col == cursor.col {
            // Reverse video: a basic, universally-supported attribute (unlike
            // underline_color) so the selected cell is always visible.
            style = style.add_modifier(Modifier::REVERSED);
        }
        cells.push(Cell::from(GRID_GLYPH).style(style));
    }

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
