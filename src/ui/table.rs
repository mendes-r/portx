use ratatui::{
    layout::Constraint,
    style::Modifier,
    style::Style,
    widgets::{Block, Borders, Row, Table},
};

fn row_highlight_style() -> Style {
    Style::new().add_modifier(Modifier::REVERSED)
}

pub fn generate_table(
    rows: Vec<Row<'static>>,
    column_count: usize,
    label_width: u16,
) -> Table<'static> {
    let widths = widths_constraints(column_count, label_width);

    Table::new(rows, widths)
        .column_spacing(0)
        .row_highlight_style(row_highlight_style())
        .highlight_symbol(">>")
        .block(Block::new().borders(Borders::ALL).title("port matrix"))
}

// Always-visible list of every currently active port/process. Unlike
// `generate_table`'s 128 single-width grid columns, these are ordinary text
// columns: port number, state label, owning process.
pub fn generate_process_table(rows: Vec<Row<'static>>) -> Table<'static> {
    let widths = [
        Constraint::Length(6),  // "65535"
        Constraint::Length(12), // "established"
        Constraint::Fill(1),    // process name, variable width
    ];
    Table::new(rows, widths)
        .header(
            Row::new(vec!["port", "state", "process"])
                .style(Style::new().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1)
        .row_highlight_style(row_highlight_style())
        .block(Block::new().borders(Borders::ALL).title("active ports"))
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
