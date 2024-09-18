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

pub fn ui(frame: &mut Frame) {
    let wrapper = wrapper(frame);

    let header_wrapper = wrapper[0];
    let matrix_wrapper = wrapper[1];

    const WIDTH: usize = 16;
    const HEIGHT: usize = 2;

    frame.render_widget(
        Paragraph::new(" # header").block(Block::new().borders(Borders::ALL)),
        header_wrapper,
    );

    let mut table_state = TableState::default();


    let vector = generate_array(WIDTH, HEIGHT);

    let mut rows: [Row<'static>; HEIGHT];

    // populate array
    for i in (0..4) {
        &rows[i] = match vector.get(i){
            None => Row::new(["failure"]),
            Some(quotient) => {
                quotient
            },
        }
    }

    let widths = [
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
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

// src: https://stackoverflow.com/questions/59164456/how-do-i-return-an-array-from-a-rust-function
fn generate_array(width: usize, height: usize) -> Vec<Row<'static>> {
        let mut arr = Vec::with_capacity(height);

        for item in arr.iter_mut() {
            let mut row = Vec::with_capacity(width);

            for _n in 1..width {
                row.push("hello");
            }

            *item = Row::new(row);
        }
        arr
    
}
