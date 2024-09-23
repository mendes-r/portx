use ratatui::{
    layout::Constraint,
    style::Modifier,
    style::Style,
    widgets::{Block, Borders, Row, Table},
};

pub fn generate_table<const WIDTH: usize>(rows: [Row<'static>; WIDTH]) -> Table<'_> {
    let widths: [Constraint; WIDTH] = widths_constraints();

    Table::new(rows, widths)
        .block(Block::new().title("matrix"))
        .column_spacing(0)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .block(Block::new().borders(Borders::ALL))
}

fn widths_constraints<const WIDTH: usize>() -> [Constraint; WIDTH] {
    let mut widths = [Constraint::Length(1); WIDTH];
    // Fill the margins to center the matrix with the real content
    widths[0] = Constraint::Fill(1);
    widths[WIDTH - 1] = Constraint::Fill(1);
    widths
}
