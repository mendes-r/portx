use std::rc::Rc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Block,
    widgets::Borders,
    widgets::Paragraph,
    Frame,
};

pub fn ui(frame: &mut Frame) {
    let wrapper = wrapper(frame);

    let header_wrapper = wrapper[0];
    let matrix_wrapper = wrapper[1];

    let width = 16;
    let height = 32 / width;

    let matrix_columns = matrix_column_layout(matrix_wrapper, width);

    // populate grid
    for n in 1..(width + 1) {
        let rows = matrix_rows_layout(matrix_columns[n as usize], height);

        for n in 0..(height - 1) {
            frame.render_widget(Block::new().borders(Borders::ALL), rows[n as usize]);
        }
    }

    frame.render_widget(
        Paragraph::new(" # header").block(Block::new().borders(Borders::ALL)),
        header_wrapper,
    );

    for n in 0..width {
        frame.render_widget(
            Block::new().borders(Borders::ALL),
            matrix_columns[n as usize + 1],
        );
    }
}

fn wrapper(frame: &mut Frame) -> Rc<[Rect]> {
    return Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(5), Constraint::Percentage(100)])
        .split(frame.area());
}

fn matrix_column_layout(parent: Rect, width: u32) -> Rc<[Rect]> {
    let mut constrains = Vec::with_capacity(width as usize + 2);
    let min_margins = 5;
    let mut n = 0;

    // left padding
    constrains.push(Constraint::Min(min_margins));

    // columns
    while n < width {
        constrains.push(Constraint::Max(3));
        n += 1;
    }

    // right padding
    constrains.push(Constraint::Min(min_margins));

    return Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constrains)
        .split(parent);
}

fn matrix_rows_layout(parent: Rect, height: u32) -> Rc<[Rect]> {
    let mut constrains = Vec::with_capacity(height as usize);
    let mut n = 0;

    // columns
    while n < height {
        constrains.push(Constraint::Ratio(1, height));
        n += 1;
    }

    return Layout::default()
        .direction(Direction::Vertical)
        .constraints(constrains)
        .split(parent);
}
