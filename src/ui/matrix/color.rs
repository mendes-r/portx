use ratatui::style::Color;

use crate::ports::PortStatus;

const ESTABLISHED: Color = Color::Rgb(200, 0, 180); // magenta
const LISTENING: Color = Color::Rgb(140, 50, 220); // purple
const UDP_ACTIVE: Color = Color::Rgb(0, 120, 220); // blue
const CLOSED: Color = Color::Rgb(40, 40, 40); // dark grey: real port, nothing active on it
const CLOSED_WELL_KNOWN: Color = Color::Rgb(60, 60, 60); // lighter grey, for closed ports 0-1023

// Amber: chosen to be visually distinct from every status color (magenta,
// purple, blue, greys) so a cell that just changed reads unambiguously even
// mid-fade.
pub const FLASH: Color = Color::Rgb(255, 210, 0);

// Contour drawn at the edge of a cell with no resolved owning process (a
// closed port, or one whose process couldn't be identified). Active cells
// with a resolved process use `process_border_color` instead.
pub const GRID_LINE: Color = Color::Black;

// Row-label text color when nothing in the row has changed recently. An RGB
// stand-in for the previous plain `Color::DarkGray`, rather than the named
// variant itself, so it can be blended toward `FLASH` via `lerp_color`
// (which only knows how to interpolate between `Color::Rgb` values).
pub const LABEL: Color = Color::Rgb(128, 128, 128);

// Fills the selected cell regardless of its own status color, so the
// cursor stays visible no matter what's underneath it.
pub const CURSOR: Color = Color::White;

// Full-detail state label for the active-ports panel: reads `tcp_state`
// directly so transitional states (CLOSE_WAIT, TIME_WAIT, ...) show up
// instead of collapsing into "closed".
pub fn status_label_full(status: &PortStatus) -> &'static str {
    if status.protocol == crate::ports::Protocol::Tcp {
        status.tcp_state.label()
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

// Border color for a cell with a resolved owning process: a deterministic,
// process-name-derived hue, so the same process reads as the same color
// everywhere it holds a port. Falls back to `GRID_LINE` for closed cells or
// an unresolved process.
pub fn process_border_color(process: Option<&str>) -> Color {
    match process {
        Some(p) => hash_to_color(process_key(p)),
        None => GRID_LINE,
    }
}

// "name (pid)" -> "name". Grouping by name rather than the full string
// (pid included) is what makes one app's whole footprint read as one color
// even when it runs as several pids (worker pools, browser renderers, etc.).
fn process_key(process: &str) -> &str {
    process
        .rsplit_once(" (")
        .map(|(name, _)| name)
        .unwrap_or(process)
}

// Hues to keep process colors out of, so a border color is never mistaken
// for a status color: FLASH (~49°, amber) and the blue/purple/magenta run
// covering UDP_ACTIVE (~207°), LISTENING (~272°) and ESTABLISHED (~306°,
// close enough to LISTENING that their margins merge into one band) — each
// padded by about 25° either side. What's left is two safe bands: yellow-
// green through cyan, and red through orange-red (the second one wrapping
// past 360°/0°).
const SAFE_HUE_RANGES: [(f64, f64); 2] = [(74.0, 182.0), (331.0, 360.0 + 24.0)];

fn hash_to_color(key: &str) -> Color {
    let span: f64 = SAFE_HUE_RANGES.iter().map(|(start, end)| end - start).sum();
    let mut offset = (fnv1a(key) % span as u64) as f64;
    let mut hue = 0.0;
    for &(start, end) in &SAFE_HUE_RANGES {
        let len = end - start;
        if offset < len {
            hue = (start + offset) % 360.0;
            break;
        }
        offset -= len;
    }
    hsl_to_rgb(hue, 0.65, 0.55) // fixed saturation/lightness; hue is the only hashed axis
}

// FNV-1a: no dependency needed, and unlike `DefaultHasher` its output is a
// fixed, specified algorithm rather than "unspecified, may change between
// Rust versions" — so the same process name maps to the same hue across
// restarts of the app, not just within one run.
fn fnv1a(s: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in s.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to_u8 = |v: f64| ((v + m) * 255.0).round() as u8;
    Color::Rgb(to_u8(r1), to_u8(g1), to_u8(b1))
}

// Linear per-channel blend, used by the flash fade. Only `Color::Rgb` is
// handled since every color in this module is `Rgb`; anything else falls
// through to `to` rather than panicking.
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            Color::Rgb(lerp_u8(r1, r2, t), lerp_u8(g1, g2, t), lerp_u8(b1, b2, t))
        }
        _ => to,
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8
}
