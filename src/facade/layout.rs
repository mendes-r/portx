#[path = "../service/ports.rs"]
mod ports;
mod table;

use std::rc::Rc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Row, TableState},
    Frame,
};

const WIDTH: usize = 25;

pub fn tui(frame: &mut Frame) {
    let wrapper = wrapper(frame);
    let header_wrapper = wrapper[0];
    let matrix_wrapper = wrapper[1];
    
    let mut table_state = TableState::default();
    let rows: [Row<'_>; WIDTH] = ports::ports_matrix();
    let table = table::generate_table(rows);
    
    frame.render_widget(
        Paragraph::new(" # header").block(Block::new().borders(Borders::ALL)),
        header_wrapper,
    );
    frame.render_stateful_widget(table, matrix_wrapper, &mut table_state);
}

fn wrapper(frame: &mut Frame) -> Rc<[Rect]> {
    return Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(5), Constraint::Percentage(100)])
        .split(frame.area());
}
