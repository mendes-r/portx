use ratatui::widgets::Row;
use std::usize;

pub fn ports_matrix<const LEN: usize>() -> [Row<'static>; LEN] {
    let matrix: [Row<'static>; LEN] = core::array::from_fn(|i| fill_row(i, LEN));
    matrix
}

fn fill_row(i: usize, width: usize) -> Row<'static> {
    let mut vec: Vec<String> = Vec::with_capacity(i);
    for n in 0..width {
        let mut content: String = i.to_string();
        content.push_str(n.to_string().as_str());

        // TODO overwrite
        // content = "%".to_owned();

        vec.push(content);
    }
    Row::new(vec)
}
