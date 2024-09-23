use rand::Rng;
use ratatui::{
    style::{Color, Style},
    widgets::{Cell, Row},
};
use std::usize;

pub fn ports_matrix<const WIDTH: usize>() -> [Row<'static>; WIDTH] {
    let matrix: [Row<'static>; WIDTH] = core::array::from_fn(|_| fill_row(WIDTH));
    matrix
}

fn fill_row(width: usize) -> Row<'static> {
    let mut vec: Vec<Cell> = Vec::with_capacity(width);

    for n in 0..(width) {
        let cell;
        if n == 0 || n == (width - 1) {
            // Margins for centering real matrix content
            cell = Cell::new("");
        } else {
            cell = Cell::from("").style(Style::default().bg(Color::Rgb(
                random_rgb(),
                random_rgb(),
                random_rgb(),
            )));
        }
        vec.push(cell);
    }
    Row::new(vec)
}

fn random_rgb() -> u8 {
    rand::thread_rng().gen_range(0..255)
}
