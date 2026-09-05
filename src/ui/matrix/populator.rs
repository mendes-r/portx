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

// Everything the selection box needs to describe the cell under the
// cursor: the real port number it renders, and the same label/color the
// cell itself is drawn with.
pub struct CellInfo {
    pub port: usize,
    pub label: &'static str,
    pub color: Color,
}

pub fn selected_cell_info(ports: &PortTable, cursor: Cursor) -> CellInfo {
    let port = cursor.row * GRID_COLS + cursor.col;
    let (label, color) = if port == HIGHLIGHTED_PORT {
        ("ssh (highlighted)", color::HIGHLIGHTED_PORT)
    } else {
        (
            color::status_label(ports[port]),
            color::status_color(ports[port]),
        )
    };

    CellInfo { port, label, color }
}

// Renders only `visible_rows` logical rows starting at `row_offset` — the
// scrolling viewport `layout::tui` positions to keep the cursor in view,
// rather than the full 512-row grid at once.
pub fn ports_matrix(
    ports: &PortTable,
    cursor: Cursor,
    row_offset: usize,
    visible_rows: usize,
) -> Vec<Row<'static>> {
    let end = (row_offset + visible_rows).min(GRID_ROWS);
    (row_offset..end)
        .map(|row| fill_row(row, ports, cursor))
        .collect()
}

fn fill_row(row: usize, ports: &PortTable, cursor: Cursor) -> Row<'static> {
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
            color::status_color(ports[port])
        };

        let mut style = grid_cell_style(bg);
        if row == cursor.row && content_col == cursor.col {
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
