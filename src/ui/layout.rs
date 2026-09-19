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

// Terminal columns needed for the smallest viable grid: the port-range
// label column, the minimum real-port columns, and a 1-column margin on the
// right. The grid itself can grow wider than this floor on a roomier
// terminal (see `matrix_populator::grid_cols`). Unlike width, height isn't
// required up front — the grid scrolls, so any number of rows renders a
// partial view of it.
const MIN_WIDTH: u16 =
    matrix_populator::LABEL_WIDTH as u16 + matrix_populator::MIN_GRID_COLS as u16 + 1;

// Fixed height for the always-visible active-ports panel: 2 borders + 1
// header row + 7 data rows.
const PANEL_HEIGHT: u16 = 10;

// Height for the bordered legend box: 2 borders + 1 content row.
const LEGEND_HEIGHT: u16 = 3;

// Smallest usable height: the grid's own border plus one content row, the
// active-ports panel, and the legend box.
const MIN_HEIGHT: u16 = 2 + 1 + PANEL_HEIGHT + LEGEND_HEIGHT;

// Grid width policy: either fit the terminal automatically, or pin it to
// one of the standard power-of-two sizes regardless of terminal width
// (clamped down if it doesn't actually fit — see `tui()`). Cycled by the
// `w` key.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum WidthMode {
    #[default]
    Auto,
    Fixed(usize),
}

impl WidthMode {
    const FIXED_STEPS: [usize; 4] = [128, 256, 512, 1024];

    pub fn cycle(&mut self) {
        *self = match self {
            WidthMode::Auto => WidthMode::Fixed(Self::FIXED_STEPS[0]),
            WidthMode::Fixed(current) => {
                let next = Self::FIXED_STEPS
                    .iter()
                    .position(|&c| c == *current)
                    .and_then(|idx| Self::FIXED_STEPS.get(idx + 1));
                match next {
                    Some(&cols) => WidthMode::Fixed(cols),
                    None => WidthMode::Auto,
                }
            }
        };
    }
}

// Selected cell in the matrix. `col` is a column into the current
// `grid_cols`-wide grid, but `row` addresses the *displayed* rows rather
// than raw grid rows: runs of all-closed rows collapse into a single
// displayed row, so the row count shrinks and grows with port state instead
// of being the fixed row count.
#[derive(Clone, Copy)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    // Current grid width, refreshed each frame in `tui()` from the
    // terminal's actual size. `right()` and `selected_port_info()` read it
    // here since they're called from `main.rs`, outside the render path,
    // where there's no `Frame`/`area` to recompute it from directly.
    pub grid_cols: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            row: 0,
            col: 0,
            grid_cols: matrix_populator::MIN_GRID_COLS,
        }
    }
}

impl Cursor {
    pub fn up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn down(&mut self, ports: &PortTable, last_changed: &[Option<Instant>]) {
        let max_row = matrix_populator::display_row_count(ports, last_changed, self.grid_cols)
            .saturating_sub(1);
        self.row = (self.row + 1).min(max_row);
    }

    pub fn left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.col = (self.col + 1).min(self.grid_cols - 1);
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
    let port = matrix_populator::selected_port(ports, last_changed, cursor, cursor.grid_cols);
    (port as u16, ports[port].pid, ports[port].process.clone())
}

pub fn tui(
    frame: &mut Frame,
    ports: &PortTable,
    cursor: &mut Cursor,
    kill_prompt: &KillPrompt,
    last_changed: &[Option<Instant>],
    width_mode: WidthMode,
) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(too_small(area), area);
        return;
    }

    let wrapper = tui_wrapper(frame);
    let legend_wp = wrapper[0];
    let matrix_wp = wrapper[1];
    let panel_wp = wrapper[2];

    // Recomputed every frame from the current terminal width so the grid
    // widens on a roomier terminal instead of staying pinned to the floor —
    // unless `width_mode` pins it to a fixed power of two, in which case
    // that's used instead as long as it actually fits (never wider than
    // what auto-fit would allow).
    let auto_grid_cols = matrix_populator::grid_cols(area.width);
    let grid_cols = match width_mode {
        WidthMode::Auto => auto_grid_cols,
        WidthMode::Fixed(cols) => cols.min(auto_grid_cols),
    };
    cursor.grid_cols = grid_cols;
    // A stale cursor from a wider frame could otherwise point past the new,
    // narrower grid's right edge.
    cursor.col = cursor.col.min(grid_cols.saturating_sub(1));

    // Collapsing all-closed rows shrinks the row count as port state
    // changes; re-clamp here so a stale cursor from a larger grid self-heals
    // rather than pointing past the end of the (now shorter) display list.
    let total_rows = matrix_populator::display_row_count(ports, last_changed, grid_cols);
    cursor.row = cursor.row.min(total_rows.saturating_sub(1));

    // Leave room for the table's own border on each side.
    let visible_rows = (matrix_wp.height.saturating_sub(2) as usize).clamp(1, total_rows.max(1));

    // The first pinned_rows(grid_cols) display rows are always shown,
    // unscrolled; only the remainder (the "body") scrolls to keep the
    // cursor in view, within whatever height is left after the pinned rows.
    let pinned = matrix_populator::pinned_rows(grid_cols)
        .min(visible_rows)
        .min(total_rows);
    let body_capacity = visible_rows - pinned;
    let body_total = total_rows - pinned;
    let body_cursor = cursor.row.saturating_sub(pinned);
    let body_offset = scroll_offset(body_cursor, body_capacity, body_total);

    let mut table_state = TableState::default();
    let rows: Vec<Row<'_>> = matrix_populator::ports_matrix(
        ports,
        *cursor,
        body_offset,
        visible_rows,
        last_changed,
        grid_cols,
    );
    let column_count = grid_cols + 2;
    let table = table::generate_table(rows, column_count, matrix_populator::LABEL_WIDTH as u16);

    render_legend(frame, legend_wp, grid_cols);
    frame.render_stateful_widget(table, matrix_wp, &mut table_state);
    render_active_panel(frame, ports, last_changed, *cursor, panel_wp);
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
    let selected_port =
        matrix_populator::selected_port(ports, last_changed, cursor, cursor.grid_cols) as u16;
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

// Renders the legend box's own border, then splits its interior so the
// current grid width can sit right-aligned on its own, separate from the
// left-aligned keybinding text.
fn render_legend(frame: &mut Frame, area: Rect, grid_cols: usize) {
    let block = Block::new().borders(Borders::ALL).title("legend");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(11)])
        .split(inner);

    let line = Line::from(vec![
        Span::styled("listening", Style::default().fg(Color::Rgb(140, 50, 220))),
        Span::raw("   "),
        Span::styled("established", Style::default().fg(Color::Rgb(200, 0, 180))),
        Span::raw("   "),
        Span::styled("udp", Style::default().fg(Color::Blue)),
        Span::raw("   "),
        Span::styled("closed", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::raw("      arrows to move      k to kill      w to cycle width      q to quit"),
    ]);
    frame.render_widget(Paragraph::new(line), cols[0]);

    frame.render_widget(
        // Trailing space is deliberate: right-aligning pushes it flush
        // against the block's border, leaving a 1-column gap before it.
        Paragraph::new(format!("{grid_cols} cols "))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right),
        cols[1],
    );
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
            Constraint::Length(LEGEND_HEIGHT),
            Constraint::Min(1),
            Constraint::Length(PANEL_HEIGHT),
        ])
        .split(frame.area())
}
