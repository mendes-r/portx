use ratatui::style::Color;

use crate::ports::PortStatus;

const ESTABLISHED: Color = Color::Rgb(200, 0, 180); // magenta
const LISTENING: Color = Color::Rgb(140, 50, 220); // purple
const UDP_ACTIVE: Color = Color::Rgb(0, 120, 220); // blue
const CLOSED: Color = Color::Rgb(40, 40, 40); // dark grey: real port, nothing active on it

// Contour drawn at the edge of every cell so the grid itself stays visible
// over the status colors.
pub const GRID_LINE: Color = Color::Black;

// Port 22 (SSH) is always called out in red, overriding its status color.
pub const HIGHLIGHTED_PORT: Color = Color::Red;

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

pub fn status_color(status: &PortStatus) -> Color {
    if status.tcp_established {
        ESTABLISHED
    } else if status.tcp_listen {
        LISTENING
    } else if status.udp_active {
        UDP_ACTIVE
    } else {
        CLOSED
    }
}
