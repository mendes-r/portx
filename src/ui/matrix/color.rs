use ratatui::style::Color;

use crate::ports::PortStatus;

const ESTABLISHED: Color = Color::Rgb(200, 0, 180); // magenta
const LISTENING: Color = Color::Rgb(140, 50, 220); // purple
const UDP_ACTIVE: Color = Color::Rgb(0, 120, 220); // blue
const CLOSED: Color = Color::Rgb(40, 40, 40); // dark grey: real port, nothing active on it
const CLOSED_WELL_KNOWN: Color = Color::Rgb(60, 60, 60); // lighter grey, for closed ports 0-1023

// Contour drawn at the edge of every cell so the grid itself stays visible
// over the status colors.
pub const GRID_LINE: Color = Color::Black;

// Fills the selected cell regardless of its own status color, so the
// cursor stays visible no matter what's underneath it.
pub const CURSOR: Color = Color::White;

// Mirrors `status_color`'s priority (established > listening > udp > closed)
// so the selection box always names the same state the cell is colored for.
pub fn status_label(status: &PortStatus) -> &'static str {
    if status.tcp_established {
        "established"
    } else if status.tcp_listen {
        "listening"
    } else if status.udp_active {
        "udp"
    } else {
        "closed"
    }
}

pub fn status_color(status: &PortStatus, well_known: bool) -> Color {
    if status.tcp_established {
        ESTABLISHED
    } else if status.tcp_listen {
        LISTENING
    } else if status.udp_active {
        UDP_ACTIVE
    } else if well_known {
        CLOSED_WELL_KNOWN
    } else {
        CLOSED
    }
}
