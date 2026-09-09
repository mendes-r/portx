use std::process::Command;

use super::PortStatus;

pub fn scan(table: &mut [PortStatus]) {
    let Ok(output) = Command::new("lsof")
        .args(["-nP", "-iTCP", "-iUDP"])
        .output()
    else {
        return;
    };
    let Ok(text) = String::from_utf8(output.stdout) else {
        return;
    };

    // Columns: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
    // NODE holds the protocol (TCP/UDP); NAME can itself contain spaces
    // (e.g. "127.0.0.1:54321->93.184.216.34:443 (ESTABLISHED)").
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 9 {
            continue;
        }

        let command = fields[0];
        let pid = fields[1];
        let proto = fields[7];
        let name = fields[8..].join(" ");
        let Some(port) = local_port(&name) else {
            continue;
        };

        match proto {
            "TCP" => {
                if name.contains("LISTEN") {
                    table[port as usize].tcp_listen = true;
                    if is_wildcard_local(&name) {
                        table[port as usize].bind_global = true;
                    }
                }
                if name.contains("ESTABLISHED") {
                    table[port as usize].tcp_established = true;
                }
            }
            "UDP" => {
                table[port as usize].udp_active = true;
                if is_wildcard_local(&name) {
                    table[port as usize].bind_global = true;
                }
            }
            _ => continue,
        }
        table[port as usize].process = Some(format!("{command} ({pid})"));
        table[port as usize].pid = pid.parse().ok();
    }
}

fn local_addr(name: &str) -> Option<&str> {
    let local = name.split("->").next()?;
    local.split_whitespace().next()
}

fn local_port(name: &str) -> Option<u16> {
    local_addr(name)?.rsplit(':').next()?.parse().ok()
}

// lsof prints the wildcard bind as a bare `*` regardless of address family,
// e.g. "*:5432 (LISTEN)" vs "127.0.0.1:5432 (LISTEN)".
fn is_wildcard_local(name: &str) -> bool {
    local_addr(name).is_some_and(|addr| addr.starts_with('*'))
}
