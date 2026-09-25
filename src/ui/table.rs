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
        Constraint::Length(5),  // "proto" header / "tcp"/"udp" values
        Constraint::Length(12), // "established", "close_wait", ...
        Constraint::Length(7),  // "global"/"local"
        Constraint::Length(4),  // "in"/"out"/"-"
        Constraint::Length(20), // process name
        Constraint::Length(10), // owner/username
        Constraint::Fill(1),    // remote "ip:port", variable width
    ];
    Table::new(rows, widths)
        .header(
            Row::new(vec![
                "port", "proto", "state", "scope", "dir", "process", "owner", "remote",
            ])
            .style(Style::new().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1)
        .row_highlight_style(row_highlight_style())
        .block(Block::new().borders(Borders::ALL).title("active ports"))
}

fn widths_constraints(column_count: usize, label_width: u16) -> Vec<Constraint> {
    let mut widths = vec![Constraint::Length(1); column_count];
    // Left margin: paired with the identical Fill(1) on the right below,
    // so ratatui splits whatever space the grid doesn't use evenly between
    // them — centering the label + grid block in the table area instead of
    // pinning it flush left.
    if let Some(first) = widths.first_mut() {
        *first = Constraint::Fill(1);
    }
    if let Some(label) = widths.get_mut(1) {
        *label = Constraint::Length(label_width);
    }
    // Index 2, the one-column gap between the (right-aligned) label and
    // the grid, is left at its default `Length(1)` — same width as every
    // grid column, just always rendered blank (see `populator::fill_row`).
    // Right margin to keep the grid off the table's border.
    if let Some(last) = widths.last_mut() {
        *last = Constraint::Fill(1);
    }
    widths
}
