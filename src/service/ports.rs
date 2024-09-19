use ratatui::widgets::Row;

pub fn ports_states() -> [Row<'static>; 256] {
    let mut arr = Vec::with_capacity(256);

    for item in arr.iter_mut() {
        let mut row = Vec::with_capacity(256);

        for _n in 1..256 {
            row.push("hello");
        }

        *item = Row::new(row);
    }
    arr

}