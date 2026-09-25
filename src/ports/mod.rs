// Prior art consulted while writing the platform scanners:
// https://github.com/RustScan/RustScan/blob/master/src/scanner/socket_iterator.rs
// https://github.com/F1bonacc1/netstat/blob/master/netstat/netstat_linux.go
// https://github.com/theopfr/somo

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TcpState {
    #[default]
    None,
    Listen,
    Established,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    TimeWait,
    Close,
    CloseWait,
    LastAck,
    Closing,
}

impl TcpState {
    pub fn label(self) -> &'static str {
        match self {
            TcpState::None => "-",
            TcpState::Listen => "listen",
            TcpState::Established => "established",
            TcpState::SynSent => "syn_sent",
            TcpState::SynRecv => "syn_recv",
            TcpState::FinWait1 => "fin_wait1",
            TcpState::FinWait2 => "fin_wait2",
            TcpState::TimeWait => "time_wait",
            TcpState::Close => "close",
            TcpState::CloseWait => "close_wait",
            TcpState::LastAck => "last_ack",
            TcpState::Closing => "closing",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Protocol {
    #[default]
    None,
    Tcp,
    Udp,
}

// Whether a connection was initiated by this machine ("outbound", we
// dialed out) or by a remote peer ("inbound", someone connected to us).
// UDP is connectionless and never gets past `Unknown`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Unknown,
    Inbound,
    Outbound,
}

impl Direction {
    pub fn label(self) -> &'static str {
        match self {
            Direction::Unknown => "-",
            Direction::Inbound => "in",
            Direction::Outbound => "out",
        }
    }
}

// Best-effort inference of connection direction. The kernel only records
// who dialed while the handshake is in flight (SYN_SENT = we sent the
// SYN, SYN_RECV = we answered one), so once a connection reaches
// ESTABLISHED (or later, e.g. CLOSE_WAIT) there's no direct flag left to
// read. Fall back to the well-known-port convention instead: servers
// typically listen on a low, well-known local port while the initiating
// side dials out from a high ephemeral one. This is a heuristic, not a
// guarantee — a server on a high port (common for dev tools) or a
// high-port-to-high-port connection reads as `Unknown` rather than
// guessed wrong.
pub fn infer_tcp_direction(
    state: TcpState,
    local_port: u16,
    remote_addr: Option<&str>,
) -> Direction {
    match state {
        TcpState::SynSent => return Direction::Outbound,
        TcpState::SynRecv => return Direction::Inbound,
        TcpState::Listen | TcpState::None => return Direction::Unknown,
        _ => {}
    }

    let remote_port = remote_addr
        .and_then(|addr| addr.rsplit(':').next())
        .and_then(|p| p.parse::<u16>().ok());

    match remote_port {
        Some(remote_port) if local_port < 1024 && remote_port >= 1024 => Direction::Inbound,
        Some(remote_port) if remote_port < 1024 && local_port >= 1024 => Direction::Outbound,
        _ => Direction::Unknown,
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct PortStatus {
    pub tcp_listen: bool,
    pub tcp_established: bool,
    pub udp_active: bool,
    // True when the LISTEN/UDP scan line for this port showed the wildcard
    // local address (0.0.0.0/:: on Linux, `*` on macOS) rather than a
    // loopback/specific-interface bind. Never set from an ESTABLISHED line:
    // a connected socket's local address is always concrete once the
    // handshake completes, so "bound wide open" isn't meaningful there.
    // Meaningful only when `tcp_listen` or `udp_active` is also set;
    // rendering code re-checks that before using it.
    pub bind_global: bool,
    // Process backing whichever state above is set, formatted as "name (pid)".
    // Best-effort: left `None` when the owning process can't be resolved
    // (e.g. it belongs to another user and we lack permission to inspect it).
    pub process: Option<String>,
    // Same process, as a raw pid, so it can be signaled (see `terminate`/
    // `force_kill`) without re-parsing the display string above.
    pub pid: Option<u32>,
    // Full TCP state (superset of `tcp_listen`/`tcp_established`, which stay
    // as a derived compatibility layer for the grid's rendering code). Left
    // at its default (`None`) for UDP rows.
    pub tcp_state: TcpState,
    pub protocol: Protocol,
    // "ip:port" of the remote peer. Only meaningful for TCP rows with an
    // actual connection (e.g. established); `None` for LISTEN rows, which
    // have no peer yet.
    pub remote_addr: Option<String>,
    pub uid: Option<u32>,
    // Resolved from `uid` where possible; `None` if the lookup fails.
    pub owner: Option<String>,
    // See `infer_tcp_direction`. Stays `Unknown` for UDP and closed ports.
    pub direction: Direction,
}

pub type PortTable = Vec<PortStatus>;

pub const PORT_COUNT: usize = 65536;

pub fn scan() -> PortTable {
    let mut table = vec![PortStatus::default(); PORT_COUNT];

    #[cfg(target_os = "linux")]
    linux::scan(&mut table);
    #[cfg(target_os = "macos")]
    macos::scan(&mut table);

    table
}

// Sends SIGTERM, giving the process a chance to shut its ports down
// cleanly. There's no way to close a single socket independently of the
// process that owns it (Linux's `ss -K` can do that for a single TCP
// socket, but it's Linux/TCP-only) — the process is the closest thing to a
// unit of "close this port" that exists on every platform this runs on.
pub fn terminate(pid: u32) -> std::io::Result<()> {
    send_signal(pid, libc::SIGTERM)
}

// Un-ignorable fallback for a process that didn't exit after `terminate`.
pub fn force_kill(pid: u32) -> std::io::Result<()> {
    send_signal(pid, libc::SIGKILL)
}

fn send_signal(pid: u32, signal: i32) -> std::io::Result<()> {
    // Safety: `kill(2)` only reads its arguments; passing a stale/unowned
    // pid just fails with ESRCH/EPERM rather than causing UB.
    let result = unsafe { libc::kill(pid as i32, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
