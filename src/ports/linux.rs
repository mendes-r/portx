use std::collections::HashMap;
use std::fs;

use super::PortStatus;

pub fn scan(table: &mut [PortStatus]) {
    let inodes = build_inode_map();
    scan_tcp("/proc/net/tcp", table, &inodes);
    scan_tcp("/proc/net/tcp6", table, &inodes);
    scan_udp("/proc/net/udp", table, &inodes);
    scan_udp("/proc/net/udp6", table, &inodes);
}

fn scan_tcp(path: &str, table: &mut [PortStatus], inodes: &HashMap<u64, (String, u32)>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(port) = local_port(&fields) else {
            continue;
        };
        match fields.get(3) {
            Some(&"0A") => {
                table[port as usize].tcp_listen = true;
                if is_wildcard(&fields) {
                    table[port as usize].bind_global = true;
                }
            }
            Some(&"01") => table[port as usize].tcp_established = true,
            _ => continue,
        }
        set_process(&mut table[port as usize], &fields, inodes);
    }
}

fn scan_udp(path: &str, table: &mut [PortStatus], inodes: &HashMap<u64, (String, u32)>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if let Some(port) = local_port(&fields) {
            table[port as usize].udp_active = true;
            if is_wildcard(&fields) {
                table[port as usize].bind_global = true;
            }
            set_process(&mut table[port as usize], &fields, inodes);
        }
    }
}

fn set_process(status: &mut PortStatus, fields: &[&str], inodes: &HashMap<u64, (String, u32)>) {
    if let Some(inode) = socket_inode(fields) {
        if let Some((process, pid)) = inodes.get(&inode) {
            status.process = Some(process.clone());
            status.pid = Some(*pid);
        }
    }
}

// `local_address` (field 1) is formatted as `HEXIP:HEXPORT`.
fn local_addr_hex(fields: &[&str]) -> Option<&str> {
    fields.get(1)?.split(':').next()
}

fn local_port(fields: &[&str]) -> Option<u16> {
    let addr = fields.get(1)?;
    let port_hex = addr.split(':').nth(1)?;
    u16::from_str_radix(port_hex, 16).ok()
}

// All-zero IP hex (8 chars for IPv4 rows in tcp/udp, 32 for IPv6 rows in
// tcp6/udp6) is the kernel's encoding of 0.0.0.0 / ::.
fn is_wildcard(fields: &[&str]) -> bool {
    local_addr_hex(fields).is_some_and(|hex| hex.chars().all(|c| c == '0'))
}

// Field 9 (0-indexed) of a `/proc/net/{tcp,udp}` row is the socket's inode,
// which cross-references `/proc/[pid]/fd/*` symlinks (see `build_inode_map`)
// to find the owning process.
fn socket_inode(fields: &[&str]) -> Option<u64> {
    fields.get(9)?.parse().ok()
}

// Maps a socket's inode to its owning process, formatted as "name (pid)".
// Built by walking every process's open file descriptors looking for
// `socket:[inode]` symlinks — the same technique tools like `ss -tunlp` and
// `lsof` use under the hood. Best-effort: processes owned by other users are
// silently skipped rather than erroring, since reading their `fd` directory
// requires privileges we may not have.
fn build_inode_map() -> HashMap<u64, (String, u32)> {
    let mut map = HashMap::new();
    let Ok(proc_dir) = fs::read_dir("/proc") else {
        return map;
    };

    for entry in proc_dir.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(fds) = fs::read_dir(entry.path().join("fd")) else {
            continue;
        };

        for fd in fds.flatten() {
            let Ok(link) = fs::read_link(fd.path()) else {
                continue;
            };
            let Some(inode) = link
                .to_str()
                .and_then(|s| s.strip_prefix("socket:["))
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.parse().ok())
            else {
                continue;
            };
            map.entry(inode).or_insert_with(|| (process_name(pid), pid));
        }
    }

    map
}

fn process_name(pid: u32) -> String {
    let name = fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "?".to_string());
    format!("{name} ({pid})")
}
