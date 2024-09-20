use ratatui::widgets::Row;
use std::usize;

const WIDTH: usize = 10;

pub fn ports_matrix() -> [Row<'static>; WIDTH] {
    let array: [Row<'static>; WIDTH] = core::array::from_fn(cb);
    array
}

fn cb(i: usize) -> Row<'static> {
    let mut vec: Vec<String> = Vec::with_capacity(WIDTH);
    for _n in 0..10 {
        vec.push(i.to_string());
    }

    Row::new(vec)
}
