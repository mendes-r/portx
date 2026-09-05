use std::fs;

use super::PortStatus;

pub fn scan(table: &mut [PortStatus]) {
    scan_tcp("/proc/net/tcp", table);
    scan_tcp("/proc/net/tcp6", table);
    scan_udp("/proc/net/udp", table);
    scan_udp("/proc/net/udp6", table);
}

fn scan_tcp(path: &str, table: &mut [PortStatus]) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(port) = local_port(&fields) else {
            continue;
        };
        match fields.get(3) {
            Some(&"0A") => table[port as usize].tcp_listen = true,
            Some(&"01") => table[port as usize].tcp_established = true,
            _ => {}
        }
    }
}

fn scan_udp(path: &str, table: &mut [PortStatus]) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if let Some(port) = local_port(&fields) {
            table[port as usize].udp_active = true;
        }
    }
}

// `local_address` (field 1) is formatted as `HEXIP:HEXPORT`.
fn local_port(fields: &[&str]) -> Option<u16> {
    let addr = fields.get(1)?;
    let port_hex = addr.split(':').nth(1)?;
    u16::from_str_radix(port_hex, 16).ok()
}
