use std::process::Command;

use super::{PortStatus, Protocol, TcpState};

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
        let user = fields[2];
        let proto = fields[7];
        let name = fields[8..].join(" ");
        let Some(port) = local_port(&name) else {
            continue;
        };

        let status = &mut table[port as usize];
        match proto {
            "TCP" => {
                let state = tcp_state(&name);
                status.tcp_state = state;
                status.protocol = Protocol::Tcp;
                status.tcp_listen = matches!(state, TcpState::Listen);
                status.tcp_established = matches!(state, TcpState::Established);
                if state == TcpState::Listen && is_wildcard_local(&name) {
                    status.bind_global = true;
                }
                status.remote_addr = remote_addr(&name);
            }
            "UDP" => {
                status.udp_active = true;
                status.protocol = Protocol::Udp;
                if is_wildcard_local(&name) {
                    status.bind_global = true;
                }
            }
            _ => continue,
        }
        status.process = Some(format!("{command} ({pid})"));
        status.pid = pid.parse().ok();
        status.owner = Some(user.to_string());
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

// The remote half of "local->remote (STATE)", with the trailing state
// stripped. `None` when there's no `->` at all (e.g. a LISTEN row, which
// has no peer).
fn remote_addr(name: &str) -> Option<String> {
    let (_, rest) = name.split_once("->")?;
    rest.split_whitespace().next().map(str::to_string)
}

// lsof embeds the TCP state as a parenthesized token at the end of NAME,
// e.g. "(ESTABLISHED)", "(CLOSE_WAIT)". Unrecognized/missing tokens (UDP
// rows have none) map to `TcpState::None`.
fn tcp_state(name: &str) -> TcpState {
    let Some(start) = name.rfind('(') else {
        return TcpState::None;
    };
    let token = name[start + 1..].trim_end_matches(')');
    match token {
        "LISTEN" => TcpState::Listen,
        "ESTABLISHED" => TcpState::Established,
        "SYN_SENT" => TcpState::SynSent,
        "SYN_RCVD" => TcpState::SynRecv,
        "FIN_WAIT_1" => TcpState::FinWait1,
        "FIN_WAIT_2" => TcpState::FinWait2,
        "TIME_WAIT" => TcpState::TimeWait,
        "CLOSED" => TcpState::Close,
        "CLOSE_WAIT" => TcpState::CloseWait,
        "LAST_ACK" => TcpState::LastAck,
        "CLOSING" => TcpState::Closing,
        _ => TcpState::None,
    }
}
