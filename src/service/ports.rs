use ratatui::{
    widgets::{Cell, Row},
    style::{Color, Style},
};
use std::usize;
use rand::Rng;

pub fn ports_matrix<const WIDTH: usize>() -> [Row<'static>; WIDTH] {
    let matrix: [Row<'static>; WIDTH] = core::array::from_fn(|_| fill_row(WIDTH));
    matrix
}

fn fill_row(width: usize) -> Row<'static> {
    let mut vec: Vec<Cell> = Vec::with_capacity(width);
    for _n in 0..width {
        let cell = Cell::from("").style(Style::default().bg(Color::Rgb(random_rgb(), random_rgb(), random_rgb())));
        vec.push(cell);
    }
    Row::new(vec)
}

fn random_rgb() -> u8 {
    rand::thread_rng().gen_range(0..255)
}