use std::collections::HashMap;
use std::fs;

use super::{PortStatus, Protocol, TcpState};

pub fn scan(table: &mut [PortStatus]) {
    let inodes = build_inode_map();
    let mut usernames = HashMap::new();
    scan_tcp("/proc/net/tcp", table, &inodes, &mut usernames);
    scan_tcp("/proc/net/tcp6", table, &inodes, &mut usernames);
    scan_udp("/proc/net/udp", table, &inodes, &mut usernames);
    scan_udp("/proc/net/udp6", table, &inodes, &mut usernames);
}

// Maps the hex TCP state code from field 3 of a `/proc/net/tcp` line to the
// full state. See `include/net/tcp_states.h` in the kernel source for the
// canonical enum this mirrors.
fn tcp_state_from_hex(hex: &str) -> Option<TcpState> {
    match hex {
        "01" => Some(TcpState::Established),
        "02" => Some(TcpState::SynSent),
        "03" => Some(TcpState::SynRecv),
        "04" => Some(TcpState::FinWait1),
        "05" => Some(TcpState::FinWait2),
        "06" => Some(TcpState::TimeWait),
        "07" => Some(TcpState::Close),
        "08" => Some(TcpState::CloseWait),
        "09" => Some(TcpState::LastAck),
        "0A" => Some(TcpState::Listen),
        "0B" => Some(TcpState::Closing),
        _ => None,
    }
}

fn scan_tcp(
    path: &str,
    table: &mut [PortStatus],
    inodes: &HashMap<u64, (String, u32)>,
    usernames: &mut HashMap<u32, Option<String>>,
) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(port) = local_port(&fields) else {
            continue;
        };
        let Some(state) = fields.get(3).and_then(|s| tcp_state_from_hex(s)) else {
            continue;
        };

        let status = &mut table[port as usize];
        status.tcp_state = state;
        status.protocol = Protocol::Tcp;
        status.tcp_listen = matches!(state, TcpState::Listen);
        status.tcp_established = matches!(state, TcpState::Established);
        if state == TcpState::Listen && is_wildcard(&fields) {
            status.bind_global = true;
        }
        status.remote_addr = remote_addr(&fields);
        set_process(status, &fields, inodes);
        set_owner(status, &fields, usernames);
    }
}

fn scan_udp(
    path: &str,
    table: &mut [PortStatus],
    inodes: &HashMap<u64, (String, u32)>,
    usernames: &mut HashMap<u32, Option<String>>,
) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if let Some(port) = local_port(&fields) {
            let status = &mut table[port as usize];
            status.udp_active = true;
            status.protocol = Protocol::Udp;
            if is_wildcard(&fields) {
                status.bind_global = true;
            }
            set_process(status, &fields, inodes);
            set_owner(status, &fields, usernames);
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

fn set_owner(
    status: &mut PortStatus,
    fields: &[&str],
    usernames: &mut HashMap<u32, Option<String>>,
) {
    let Some(uid) = fields.get(7).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    status.uid = Some(uid);
    status.owner = usernames
        .entry(uid)
        .or_insert_with(|| username_for_uid(uid))
        .clone();
}

// Safety: `getpwuid`'s `pw_name` points into libc-owned static/thread-local
// storage that's overwritten on the next call — copy it out immediately and
// never retain the pointer past this function.
fn username_for_uid(uid: u32) -> Option<String> {
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return None;
        }
        Some(
            std::ffi::CStr::from_ptr((*pw).pw_name)
                .to_string_lossy()
                .into_owned(),
        )
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

// `rem_address` (field 2) is formatted the same way as `local_address`:
// `HEXIP:HEXPORT`, little-endian per 32-bit word. `None` when there's no
// peer yet (all-zero, e.g. a LISTEN row) or the field can't be parsed.
fn remote_addr(fields: &[&str]) -> Option<String> {
    let field = fields.get(2)?;
    let (ip_hex, port_hex) = field.split_once(':')?;
    if ip_hex.chars().all(|c| c == '0') {
        return None;
    }
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let ip = decode_hex_ip(ip_hex)?;
    Some(format!("{ip}:{port}"))
}

// Each 4 hex chars is one little-endian 32-bit word; IPv4 is one word,
// IPv6 is four.
fn decode_hex_ip(hex: &str) -> Option<String> {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<_, _>>()
        .ok()?;

    match bytes.len() {
        4 => Some(format!(
            "{}.{}.{}.{}",
            bytes[3], bytes[2], bytes[1], bytes[0]
        )),
        16 => {
            // Each 4-byte word is little-endian; reversing within a word
            // (but not the word order) recovers network-order address bytes.
            let addr_bytes: Vec<u8> = bytes
                .chunks(4)
                .flat_map(|word| word.iter().rev().copied())
                .collect();
            let groups: Vec<String> = addr_bytes
                .chunks(2)
                .map(|pair| format!("{:02x}{:02x}", pair[0], pair[1]))
                .collect();
            Some(groups.join(":"))
        }
        _ => None,
    }
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
