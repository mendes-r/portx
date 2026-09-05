use ratatui::{
    layout::Constraint,
    style::Modifier,
    style::Style,
    widgets::{Block, Borders, Row, Table},
};

pub fn generate_table(
    rows: Vec<Row<'static>>,
    column_count: usize,
    label_width: u16,
) -> Table<'static> {
    let widths = widths_constraints(column_count, label_width);

    Table::new(rows, widths)
        .block(Block::new().title("matrix"))
        .column_spacing(0)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .block(Block::new().borders(Borders::ALL))
}

fn widths_constraints(column_count: usize, label_width: u16) -> Vec<Constraint> {
    let mut widths = vec![Constraint::Length(1); column_count];
    if let Some(first) = widths.first_mut() {
        *first = Constraint::Length(label_width);
    }
    // Fill the right margin to keep the grid off the table's border.
    if let Some(last) = widths.last_mut() {
        *last = Constraint::Fill(1);
    }
    widths
}
