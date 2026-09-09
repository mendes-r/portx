#[path = "./matrix/color.rs"]
mod color;
#[path = "./matrix/populator.rs"]
mod matrix_populator;
#[path = "./panel.rs"]
mod panel;
mod table;

use std::rc::Rc;
use std::time::Instant;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, TableState},
    Frame,
};

use crate::kill::KillPrompt;
use crate::ports::PortTable;

// Well-Known Ports (0–1023): Reserved for commonly used services and protocols
//
// Registered Ports (1024–49151): Assigned by IANA for specific services
//
// Dynamic/Private Ports (49152–65535)

// Terminal columns needed for the grid: the port-range label column, the
// 128 real ports, and a 1-column margin on the right. Unlike width, height
// isn't required up front — the grid scrolls, so any number of rows renders
// a partial view of it.
const MIN_WIDTH: u16 =
    matrix_populator::LABEL_WIDTH as u16 + matrix_populator::GRID_COLS as u16 + 1;

// Fixed height for the always-visible active-ports panel: 2 borders + 1
// header row + 7 data rows.
const PANEL_HEIGHT: u16 = 10;

// Height for the bordered legend box: 2 borders + 1 content row.
const LEGEND_HEIGHT: u16 = 3;

// Smallest usable height: the grid's own border plus one content row, the
// active-ports panel, and the legend box.
const MIN_HEIGHT: u16 = 2 + 1 + PANEL_HEIGHT + LEGEND_HEIGHT;

// Selected cell in the matrix. `col` is a column into the 128-wide grid,
// but `row` addresses the *displayed* rows rather than raw grid rows: runs
// of all-closed rows collapse into a single displayed row, so the row count
// shrinks and grows with port state instead of being the fixed GRID_ROWS.
#[derive(Default, Clone, Copy)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    pub fn up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn down(&mut self, ports: &PortTable, last_changed: &[Option<Instant>]) {
        let max_row = matrix_populator::display_row_count(ports, last_changed).saturating_sub(1);
        self.row = (self.row + 1).min(max_row);
    }

    pub fn left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.col = (self.col + 1).min(matrix_populator::GRID_COLS - 1);
    }
}

// Port, pid, and process name under the cursor, for the kill-prompt flow to
// act on. Pid is `None` when the port is closed or its owner couldn't be
// resolved — callers use that to decide whether there's anything to kill.
pub fn selected_port_info(
    ports: &PortTable,
    last_changed: &[Option<Instant>],
    cursor: Cursor,
) -> (u16, Option<u32>, Option<String>) {
    let port = matrix_populator::selected_port(ports, last_changed, cursor);
    (port as u16, ports[port].pid, ports[port].process.clone())
}

pub fn tui(
    frame: &mut Frame,
    ports: &PortTable,
    cursor: &mut Cursor,
    kill_prompt: &KillPrompt,
    last_changed: &[Option<Instant>],
) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(too_small(area), area);
        return;
    }

    let wrapper = tui_wrapper(frame);
    let matrix_wp = wrapper[0];
    let panel_wp = wrapper[1];
    let legend_wp = wrapper[2];

    // Collapsing all-closed rows shrinks the row count as port state
    // changes; re-clamp here so a stale cursor from a larger grid self-heals
    // rather than pointing past the end of the (now shorter) display list.
    let total_rows = matrix_populator::display_row_count(ports, last_changed);
    cursor.row = cursor.row.min(total_rows.saturating_sub(1));

    // Leave room for the table's own border on each side.
    let visible_rows = (matrix_wp.height.saturating_sub(2) as usize).clamp(1, total_rows.max(1));

    // The first PINNED_ROWS display rows are always shown, unscrolled;
    // only the remainder (the "body") scrolls to keep the cursor in view,
    // within whatever height is left after the pinned rows.
    let pinned = matrix_populator::PINNED_ROWS
        .min(visible_rows)
        .min(total_rows);
    let body_capacity = visible_rows - pinned;
    let body_total = total_rows - pinned;
    let body_cursor = cursor.row.saturating_sub(pinned);
    let body_offset = scroll_offset(body_cursor, body_capacity, body_total);

    let mut table_state = TableState::default();
    let rows: Vec<Row<'_>> =
        matrix_populator::ports_matrix(ports, *cursor, body_offset, visible_rows, last_changed);
    let column_count = matrix_populator::GRID_COLS + 2;
    let table = table::generate_table(rows, column_count, matrix_populator::LABEL_WIDTH as u16);

    frame.render_stateful_widget(table, matrix_wp, &mut table_state);
    render_active_panel(frame, ports, last_changed, *cursor, panel_wp);
    frame.render_widget(legend(), legend_wp);
    kill_popup(frame, kill_prompt);
}

// Always-visible table of every currently active port, replacing the old
// single-port selection box: the cursor no longer needs to sit on a cell to
// see what's running there. The row matching the cursor's selected port is
// highlighted and the view auto-scrolls to keep it visible, reusing the
// same `scroll_offset` centering logic the grid's own body scroll uses.
fn render_active_panel(
    frame: &mut Frame,
    ports: &PortTable,
    last_changed: &[Option<Instant>],
    cursor: Cursor,
    area: Rect,
) {
    let selected_port = matrix_populator::selected_port(ports, last_changed, cursor) as u16;
    let entries = panel::active_entries(ports);
    let selected = panel::selected_index(&entries, selected_port);

    // Leave room for the table's border (2) and header row (1).
    let visible = area.height.saturating_sub(3) as usize;
    let offset = scroll_offset(selected.unwrap_or(0), visible, entries.len());

    let table = table::generate_process_table(panel::rows(&entries));
    let mut state = TableState::new()
        .with_offset(offset)
        .with_selected(selected);
    frame.render_stateful_widget(table, area, &mut state);
}

// Keeps the cursor roughly centered in the viewport, clamped so the view
// never scrolls past the top or bottom of the grid.
fn scroll_offset(cursor_row: usize, visible_rows: usize, total_rows: usize) -> usize {
    if visible_rows >= total_rows {
        return 0;
    }
    let half = visible_rows / 2;
    cursor_row
        .saturating_sub(half)
        .min(total_rows - visible_rows)
}

fn legend() -> Paragraph<'static> {
    let line = Line::from(vec![
        Span::styled("listening", Style::default().fg(Color::Rgb(140, 50, 220))),
        Span::raw("   "),
        Span::styled("established", Style::default().fg(Color::Rgb(200, 0, 180))),
        Span::raw("   "),
        Span::styled("udp", Style::default().fg(Color::Blue)),
        Span::raw("   "),
        Span::styled("closed", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::raw("      arrows to move      k to kill      q to quit"),
    ]);
    Paragraph::new(line).block(Block::new().borders(Borders::ALL).title("legend"))
}

// Centered modal for the kill-confirmation flow; renders nothing while
// `kill_prompt` is `KillPrompt::None`.
fn kill_popup(frame: &mut Frame, kill_prompt: &KillPrompt) {
    let text = match kill_prompt {
        KillPrompt::None => return,
        KillPrompt::ConfirmTerm(target) => format!(
            "send SIGTERM to {} (pid {})?   y/n",
            target.process, target.pid
        ),
        KillPrompt::WaitingForExit { target, .. } => {
            format!(
                "waiting for {} (pid {}) to exit...",
                target.process, target.pid
            )
        }
        KillPrompt::ConfirmKill(target) => format!(
            "{} (pid {}) is still running. send SIGKILL?   y/n",
            target.process, target.pid
        ),
        KillPrompt::Message(text) => format!("{text}   (press any key)"),
    };

    let area = centered_rect(frame.area(), text.len() as u16 + 4, 3);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::new().borders(Borders::ALL).title("kill process")),
        area,
    );
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn too_small(area: Rect) -> Paragraph<'static> {
    let lines = vec![
        Line::from("terminal too small").alignment(Alignment::Center),
        Line::from(format!(
            "need at least {MIN_WIDTH} columns x {MIN_HEIGHT} rows to show the grid"
        ))
        .alignment(Alignment::Center),
        Line::from(format!("current size: {}x{}", area.width, area.height))
            .alignment(Alignment::Center),
    ];
    Paragraph::new(lines)
}

fn tui_wrapper(frame: &mut Frame) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Length(PANEL_HEIGHT),
            Constraint::Length(LEGEND_HEIGHT),
        ])
        .split(frame.area())
}
