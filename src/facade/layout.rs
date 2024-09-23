#[path = "../service/ports.rs"]
mod ports;
mod table;

use std::rc::Rc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Row, TableState},
    Frame,
};

const WIDTH: usize = 100 + 2; // Columns + 2 for the margins

pub fn tui(frame: &mut Frame) {
    let wrapper = tui_wrapper(frame);
    let header_wp = wrapper[0];
    let main_wp = matrix_wrapper(wrapper[1]);
    let matrix_wp = main_wp[1];

    let mut table_state = TableState::default();
    let rows: [Row<'_>; WIDTH] = ports::ports_matrix();
    let table = table::generate_table(rows);

    frame.render_widget(
        Paragraph::new("header").block(Block::new().borders(Borders::ALL)),
        header_wp,
    );

    frame.render_widget(
        Paragraph::new("left-margin").block(Block::new().borders(Borders::ALL)),
        main_wp[0],
    );
    frame.render_widget(
        Paragraph::new("right-margin").block(Block::new().borders(Borders::ALL)),
        main_wp[2],
    );

    frame.render_stateful_widget(table, matrix_wp, &mut table_state);
}

fn tui_wrapper(frame: &mut Frame) -> Rc<[Rect]> {
    return Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(5), Constraint::Percentage(100)])
        .split(frame.area());
}

fn matrix_wrapper(parent: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Fill(1),
            Constraint::Percentage(75),
            Constraint::Fill(1),
        ])
        .split(parent)
}
