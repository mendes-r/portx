use ratatui::{
    layout::Constraint,
    style::Modifier,
    style::Style,
    widgets::{Block, Borders, Row, Table},
};

pub fn generate_table(rows: Vec<Row<'static>>, width: usize) -> Table<'static> {
    let widths = widths_constraints(width);

    Table::new(rows, widths)
        .block(Block::new().title("matrix"))
        .column_spacing(0)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .block(Block::new().borders(Borders::ALL))
}

fn widths_constraints(width: usize) -> Vec<Constraint> {
    let mut widths = vec![Constraint::Length(1); width];
    // Fill the margins to center the matrix with the real content
    if let Some(first) = widths.first_mut() {
        *first = Constraint::Fill(1);
    }
    if let Some(last) = widths.last_mut() {
        *last = Constraint::Fill(1);
    }
    widths
}
