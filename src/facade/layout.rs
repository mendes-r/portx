#[path = "../service/ports.rs"]
mod ports;

use std::rc::Rc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    style::Style,
    widgets::Block,
    widgets::Borders,
    widgets::Paragraph,
    widgets::Row,
    widgets::Table,
    widgets::TableState,
    Frame,
};

pub fn generate_ui(frame: &mut Frame) {

    let wrapper = wrapper(frame);
    let header_wrapper = wrapper[0];
    let matrix_wrapper = wrapper[1];

    frame.render_widget(
        Paragraph::new(" # header").block(Block::new().borders(Borders::ALL)),
        header_wrapper,
    );

    let mut table_state = TableState::default();

    let mut content = ports::ports_states();

    let widths = [
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(10),
    ];

    let table = Table::new(content, widths)
        .block(Block::new().title("matrix"))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>");

    frame.render_stateful_widget(table, matrix_wrapper, &mut table_state);
    
}

fn wrapper(frame: &mut Frame) -> Rc<[Rect]> {
    return Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(5), Constraint::Percentage(100)])
        .split(frame.area());
}


