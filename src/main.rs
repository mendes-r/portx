#[path = "kill.rs"]
mod kill;
#[path = "ui/layout.rs"]
mod layout;
mod ports;

use std::{
    io::{self, stdout},
    time::{Duration, Instant},
};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    Terminal,
};

use kill::{KillPrompt, Target};
use layout::{Cursor, WidthMode};

// Port state changes slowly and scanning it isn't free, so it's only
// refreshed on this cadence rather than every frame — that would otherwise
// throttle arrow-key responsiveness to the same rate.
const SCAN_INTERVAL: Duration = Duration::from_millis(500);

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut cursor = Cursor::default();
    let mut port_table = ports::scan();
    let mut last_changed: Vec<Option<Instant>> = vec![None; ports::PORT_COUNT];
    let mut last_scan = Instant::now();
    let mut kill_prompt = KillPrompt::default();
    let mut width_mode = WidthMode::default();

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            layout::tui(
                frame,
                &port_table,
                &mut cursor,
                &kill_prompt,
                &last_changed,
                width_mode,
            )
        })?;
        should_quit = handle_events(
            &mut cursor,
            &port_table,
            &last_changed,
            &mut kill_prompt,
            &mut width_mode,
        )?;

        if last_scan.elapsed() >= SCAN_INTERVAL {
            let new_table = ports::scan();
            mark_changes(&port_table, &new_table, &mut last_changed);
            port_table = new_table;
            last_scan = Instant::now();
            kill_prompt.tick(&port_table);
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

// Stamps `now` on every port whose full status differs from the previous
// scan (state flags, bind scope, or owning process/pid) — drives the flash
// fade in `matrix_populator::fill_row`. `last_changed` persists across
// frames/scans (unlike `port_table`, which `ports::scan()` rebuilds from
// scratch every call), so it lives here rather than inside the scan path.
fn mark_changes(
    old: &ports::PortTable,
    new: &ports::PortTable,
    last_changed: &mut [Option<Instant>],
) {
    let now = Instant::now();
    for port in 0..new.len() {
        if old[port] != new[port] {
            last_changed[port] = Some(now);
        }
    }
}

fn handle_events(
    cursor: &mut Cursor,
    port_table: &ports::PortTable,
    last_changed: &[Option<Instant>],
    kill_prompt: &mut KillPrompt,
    width_mode: &mut WidthMode,
) -> io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if !matches!(kill_prompt, KillPrompt::None) {
                    handle_prompt_key(key.code, kill_prompt);
                    return Ok(false);
                }
                match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Up => cursor.up(),
                    KeyCode::Down => cursor.down(port_table, last_changed),
                    KeyCode::Left => cursor.left(),
                    KeyCode::Right => cursor.right(),
                    KeyCode::Char('w') => width_mode.cycle(),
                    KeyCode::Char('k') => match selected_target(cursor, port_table, last_changed) {
                        Some(target) => kill_prompt.request(target),
                        None => {
                            *kill_prompt =
                                KillPrompt::Message("no process to kill on this port".to_string())
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}

fn handle_prompt_key(code: KeyCode, kill_prompt: &mut KillPrompt) {
    match kill_prompt {
        KillPrompt::Message(_) => kill_prompt.dismiss_message(),
        KillPrompt::ConfirmTerm(_) | KillPrompt::ConfirmKill(_) => match code {
            KeyCode::Char('y') | KeyCode::Enter => kill_prompt.confirm(),
            KeyCode::Char('n') | KeyCode::Esc => kill_prompt.cancel(),
            _ => {}
        },
        KillPrompt::WaitingForExit { .. } | KillPrompt::None => {}
    }
}

fn selected_target(
    cursor: &Cursor,
    port_table: &ports::PortTable,
    last_changed: &[Option<Instant>],
) -> Option<Target> {
    let (port, pid, process) = layout::selected_port_info(port_table, last_changed, *cursor);
    let pid = pid?;
    Some(Target {
        port,
        pid,
        process: process.unwrap_or_else(|| format!("pid {pid}")),
    })
}
