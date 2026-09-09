// Prior art consulted while writing the platform scanners:
// https://github.com/RustScan/RustScan/blob/master/src/scanner/socket_iterator.rs
// https://github.com/F1bonacc1/netstat/blob/master/netstat/netstat_linux.go
// https://github.com/theopfr/somo

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

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
