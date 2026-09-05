// Prior art consulted while writing the platform scanners:
// https://github.com/RustScan/RustScan/blob/master/src/scanner/socket_iterator.rs
// https://github.com/F1bonacc1/netstat/blob/master/netstat/netstat_linux.go
// https://github.com/theopfr/somo

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[derive(Clone, Default)]
pub struct PortStatus {
    pub tcp_listen: bool,
    pub tcp_established: bool,
    pub udp_active: bool,
    // Process backing whichever state above is set, formatted as "name (pid)".
    // Best-effort: left `None` when the owning process can't be resolved
    // (e.g. it belongs to another user and we lack permission to inspect it).
    pub process: Option<String>,
}

pub type PortTable = Vec<PortStatus>;

pub fn scan() -> PortTable {
    let mut table = vec![PortStatus::default(); 65536];

    #[cfg(target_os = "linux")]
    linux::scan(&mut table);
    #[cfg(target_os = "macos")]
    macos::scan(&mut table);

    table
}
