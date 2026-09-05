#[path = "./matrix/populator.rs"]
mod matrix_populator;
mod table;

use std::rc::Rc;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, TableState},
    Frame,
};

use crate::ports::PortTable;

// Well-Known Ports (0–1023): Reserved for commonly used services and protocols
//
// Registered Ports (1024–49151): Assigned by IANA for specific services
//
// Dynamic/Private Ports (49152–65535)

// Terminal columns needed for the grid (128 real ports plus the table's own
// border on each side). Unlike width, height isn't required up front — the
// grid scrolls, so any number of rows renders a partial view of it.
const MIN_WIDTH: u16 = matrix_populator::GRID_COLS as u16 + 2;

// Smallest usable height: the grid's own border plus one content row, the
// selection box, and the legend line.
const MIN_HEIGHT: u16 = 2 + 1 + 3 + 1;

// Selected cell in the matrix. `col` is a column into the 128-wide grid,
// but `row` addresses the *displayed* rows rather than raw grid rows: runs
// of all-closed rows collapse into a single displayed row, so the row count
// shrinks and grows with port state instead of being the fixed GRID_ROWS.
#[derive(Default, Clone, Copy)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    pub fn up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn down(&mut self, ports: &PortTable) {
        let max_row = matrix_populator::display_row_count(ports).saturating_sub(1);
        self.row = (self.row + 1).min(max_row);
    }

    pub fn left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.col = (self.col + 1).min(matrix_populator::GRID_COLS - 1);
    }
}

pub fn tui(frame: &mut Frame, ports: &PortTable, cursor: &mut Cursor) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(too_small(area), area);
        return;
    }

    let wrapper = tui_wrapper(frame);
    let matrix_wp = wrapper[0];
    let selection_wp = wrapper[1];
    let legend_wp = wrapper[2];

    // Collapsing all-closed rows shrinks the row count as port state
    // changes; re-clamp here so a stale cursor from a larger grid self-heals
    // rather than pointing past the end of the (now shorter) display list.
    let total_rows = matrix_populator::display_row_count(ports);
    cursor.row = cursor.row.min(total_rows.saturating_sub(1));

    // Leave room for the table's own border on each side.
    let visible_rows = (matrix_wp.height.saturating_sub(2) as usize).clamp(1, total_rows.max(1));

    // The first PINNED_ROWS display rows are always shown, unscrolled;
    // only the remainder (the "body") scrolls to keep the cursor in view,
    // within whatever height is left after the pinned rows.
    let pinned = matrix_populator::PINNED_ROWS
        .min(visible_rows)
        .min(total_rows);
    let body_capacity = visible_rows - pinned;
    let body_total = total_rows - pinned;
    let body_cursor = cursor.row.saturating_sub(pinned);
    let body_offset = scroll_offset(body_cursor, body_capacity, body_total);

    let mut table_state = TableState::default();
    let rows: Vec<Row<'_>> =
        matrix_populator::ports_matrix(ports, *cursor, body_offset, visible_rows);
    let table = table::generate_table(rows, MIN_WIDTH as usize);

    frame.render_stateful_widget(table, matrix_wp, &mut table_state);
    frame.render_widget(selection(ports, *cursor), selection_wp);
    frame.render_widget(legend(), legend_wp);
}

// Keeps the cursor roughly centered in the viewport, clamped so the view
// never scrolls past the top or bottom of the grid.
fn scroll_offset(cursor_row: usize, visible_rows: usize, total_rows: usize) -> usize {
    if visible_rows >= total_rows {
        return 0;
    }
    let half = visible_rows / 2;
    cursor_row
        .saturating_sub(half)
        .min(total_rows - visible_rows)
}

fn selection(ports: &PortTable, cursor: Cursor) -> Paragraph<'static> {
    let info = matrix_populator::selected_cell_info(ports, cursor);

    // Only closed ports and collapsed rows are guaranteed to have no owning
    // process; every other state gets a process span, falling back to an
    // explicit "unknown" when the scanner couldn't resolve one.
    let active = matches!(info.label.as_str(), "established" | "listening" | "udp");

    let mut spans = vec![
        Span::raw(info.header),
        Span::raw("   "),
        Span::styled(info.label, Style::default().fg(info.color)),
    ];
    match info.process {
        Some(process) => {
            spans.push(Span::raw("   "));
            spans.push(Span::raw(process));
        }
        None if active => {
            spans.push(Span::raw("   "));
            spans.push(Span::styled(
                "unknown process",
                Style::default().fg(Color::DarkGray),
            ));
        }
        None => {}
    }

    Paragraph::new(Line::from(spans)).block(Block::new().borders(Borders::ALL))
}

fn legend() -> Paragraph<'static> {
    let line = Line::from(vec![
        Span::styled("listening", Style::default().fg(Color::Rgb(140, 50, 220))),
        Span::raw("   "),
        Span::styled("established", Style::default().fg(Color::Rgb(200, 0, 180))),
        Span::raw("   "),
        Span::styled("udp", Style::default().fg(Color::Blue)),
        Span::raw("   "),
        Span::styled("closed", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::raw("      arrows to move      q to quit"),
    ]);
    Paragraph::new(line)
}

fn too_small(area: Rect) -> Paragraph<'static> {
    let lines = vec![
        Line::from("terminal too small").alignment(Alignment::Center),
        Line::from(format!(
            "need at least {MIN_WIDTH} columns x {MIN_HEIGHT} rows to show the grid"
        ))
        .alignment(Alignment::Center),
        Line::from(format!("current size: {}x{}", area.width, area.height))
            .alignment(Alignment::Center),
    ];
    Paragraph::new(lines)
}

fn tui_wrapper(frame: &mut Frame) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area())
}
