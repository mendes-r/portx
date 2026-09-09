use ratatui::{
    style::Style,
    widgets::{Cell, Row},
};

use super::color;
use crate::ports::{PortStatus, PortTable};

// One row of the always-visible active-ports panel.
pub struct ActiveEntry {
    pub port: u16,
    pub status: PortStatus,
}

// Every port currently in a non-closed state, in ascending port order (the
// PortTable is already index-by-port, so a single linear pass over it
// preserves that order for free).
pub fn active_entries(ports: &PortTable) -> Vec<ActiveEntry> {
    ports
        .iter()
        .enumerate()
        .filter(|(_, s)| s.tcp_listen || s.tcp_established || s.udp_active)
        .map(|(port, s)| ActiveEntry {
            port: port as u16,
            status: s.clone(),
        })
        .collect()
}

// Index into `entries` whose port matches the cursor's currently selected
// port — `None` if the selected port isn't currently active (nothing to
// highlight in that case).
pub fn selected_index(entries: &[ActiveEntry], selected_port: u16) -> Option<usize> {
    entries.iter().position(|e| e.port == selected_port)
}

pub fn rows(entries: &[ActiveEntry]) -> Vec<Row<'static>> {
    entries
        .iter()
        .map(|e| {
            let label = color::status_label(&e.status);
            let label_color = color::status_color(&e.status, (e.port as usize) < 1024);
            let process = e
                .status
                .process
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            Row::new(vec![
                Cell::from(e.port.to_string()),
                Cell::from(label).style(Style::default().fg(label_color)),
                Cell::from(process),
            ])
        })
        .collect()
}
