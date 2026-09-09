use std::time::{Duration, Instant};

use crate::ports::{self, PortTable};

// How long to wait after SIGTERM before offering to escalate to SIGKILL —
// a couple of scan intervals, so a process that's shutting down cleanly has
// a fair chance to exit before it gets read as stuck.
const GRACE_PERIOD: Duration = Duration::from_secs(2);

// The process backing a selected port, as far as a signal is concerned:
// enough to send one and to recognize afterward whether it's still there.
#[derive(Clone)]
pub struct Target {
    pub port: u16,
    pub pid: u32,
    pub process: String,
}

// A modal confirmation/status flow for killing the process behind the
// selected port. There's no way to close a single socket independently of
// its owning process (see `ports::terminate`), so this always acts on the
// whole process — SIGTERM first, with SIGKILL offered only if it's still
// running after `GRACE_PERIOD`.
#[derive(Default)]
pub enum KillPrompt {
    #[default]
    None,
    ConfirmTerm(Target),
    WaitingForExit {
        target: Target,
        sent_at: Instant,
    },
    ConfirmKill(Target),
    Message(String),
}

impl KillPrompt {
    pub fn request(&mut self, target: Target) {
        *self = KillPrompt::ConfirmTerm(target);
    }

    pub fn cancel(&mut self) {
        *self = KillPrompt::None;
    }

    pub fn dismiss_message(&mut self) {
        if matches!(self, KillPrompt::Message(_)) {
            *self = KillPrompt::None;
        }
    }

    // Sends the signal for whichever confirmation is currently showing.
    pub fn confirm(&mut self) {
        *self = match std::mem::take(self) {
            KillPrompt::ConfirmTerm(target) => match ports::terminate(target.pid) {
                Ok(()) => KillPrompt::WaitingForExit {
                    target,
                    sent_at: Instant::now(),
                },
                Err(err) => KillPrompt::Message(format!("couldn't send SIGTERM: {err}")),
            },
            KillPrompt::ConfirmKill(target) => match ports::force_kill(target.pid) {
                Ok(()) => KillPrompt::Message(format!(
                    "sent SIGKILL to {} (pid {})",
                    target.process, target.pid
                )),
                Err(err) => KillPrompt::Message(format!("couldn't send SIGKILL: {err}")),
            },
            other => other,
        };
    }

    // Re-checked against every fresh port scan: clears the prompt once the
    // process is actually gone, or offers to escalate once `GRACE_PERIOD`
    // has passed without that happening.
    pub fn tick(&mut self, ports: &PortTable) {
        if let KillPrompt::WaitingForExit { target, sent_at } = self {
            if !still_running(ports, target) {
                *self = KillPrompt::Message(format!(
                    "{} (pid {}) terminated",
                    target.process, target.pid
                ));
            } else if sent_at.elapsed() >= GRACE_PERIOD {
                *self = KillPrompt::ConfirmKill(target.clone());
            }
        }
    }
}

fn still_running(ports: &PortTable, target: &Target) -> bool {
    ports
        .get(target.port as usize)
        .and_then(|status| status.pid)
        .is_some_and(|pid| pid == target.pid)
}
