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

// Selected cell in the matrix, addressed by logical (row, col) into the
// 128x512 port grid (row = port / 128, col = port % 128).
#[derive(Default, Clone, Copy)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    pub fn up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn down(&mut self) {
        self.row = (self.row + 1).min(matrix_populator::GRID_ROWS - 1);
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

    // Leave room for the table's own border on each side.
    let visible_rows =
        (matrix_wp.height.saturating_sub(2) as usize).clamp(1, matrix_populator::GRID_ROWS);
    let row_offset = scroll_offset(cursor.row, visible_rows);

    let mut table_state = TableState::default();
    let rows: Vec<Row<'_>> =
        matrix_populator::ports_matrix(ports, *cursor, row_offset, visible_rows);
    let table = table::generate_table(rows, MIN_WIDTH as usize);

    frame.render_stateful_widget(table, matrix_wp, &mut table_state);
    frame.render_widget(selection(ports, *cursor), selection_wp);
    frame.render_widget(legend(), legend_wp);
}

// Keeps the cursor roughly centered in the viewport, clamped so the view
// never scrolls past the top or bottom of the grid.
fn scroll_offset(cursor_row: usize, visible_rows: usize) -> usize {
    if visible_rows >= matrix_populator::GRID_ROWS {
        return 0;
    }
    let half = visible_rows / 2;
    cursor_row
        .saturating_sub(half)
        .min(matrix_populator::GRID_ROWS - visible_rows)
}

fn selection(ports: &PortTable, cursor: Cursor) -> Paragraph<'static> {
    let info = matrix_populator::selected_cell_info(ports, cursor);

    let line = Line::from(vec![
        Span::raw(format!("port {}", info.port)),
        Span::raw("   "),
        Span::styled(info.label, Style::default().fg(info.color)),
    ]);

    Paragraph::new(line).block(Block::new().borders(Borders::ALL))
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
