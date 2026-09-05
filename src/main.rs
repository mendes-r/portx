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

use layout::Cursor;

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
    let mut last_scan = Instant::now();

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| layout::tui(frame, &port_table, &mut cursor))?;
        should_quit = handle_events(&mut cursor)?;

        if last_scan.elapsed() >= SCAN_INTERVAL {
            port_table = ports::scan();
            last_scan = Instant::now();
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events(cursor: &mut Cursor) -> io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Up => cursor.up(),
                    KeyCode::Down => cursor.down(),
                    KeyCode::Left => cursor.left(),
                    KeyCode::Right => cursor.right(),
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}
