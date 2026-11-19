use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use dot_api::{Cancellation, Decision, Grant, Permission, PermissionMode, Permissions};

use crate::reply::{News, ReplyNews};

const CANCELLATION_CHECK: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Allow,
    Always,
    Deny,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Asking(Arc<Mutex<Shared>>);

#[derive(Debug, Default)]
struct Shared {
    waiting: Option<Sender<Decision>>,
    grants: Vec<Grant>,
}

pub(crate) struct Asker<'d> {
    pub(crate) id: u64,
    pub(crate) mode: PermissionMode,
    pub(crate) asking: Asking,
    pub(crate) deliver: &'d dyn Fn(ReplyNews),
}

impl Asking {
    pub(crate) fn answer(&self, answer: Answer, permission: &Permission) -> bool {
        let mut shared = self.shared();
        let Some(waiting) = shared.waiting.take() else {
            return false;
        };
        if answer == Answer::Always {
            shared.grants.push(permission.action.grant());
        }
        let decision = match answer {
            Answer::Allow | Answer::Always => Decision::Allow,
            Answer::Deny => Decision::Deny,
        };
        waiting.send(decision).is_ok()
    }

    fn shared(&self) -> MutexGuard<'_, Shared> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Permissions for Asker<'_> {
    fn ask(&self, permission: &Permission, cancellation: &Cancellation) -> Decision {
        if self.mode == PermissionMode::Bypass {
            return if cancellation.is_cancelled() {
                Decision::Deny
            } else {
                Decision::Allow
            };
        }

        let answers = {
            let mut shared = self.asking.shared();
            if shared.grants.contains(&permission.action.grant()) {
                return Decision::Allow;
            }
            let (sender, answers) = mpsc::channel();
            shared.waiting = Some(sender);
            answers
        };
        (self.deliver)(ReplyNews {
            id: self.id,
            news: News::Asked(permission.clone()),
        });

        loop {
            match answers.recv_timeout(CANCELLATION_CHECK) {
                Ok(decision) => return decision,
                Err(RecvTimeoutError::Timeout) if !cancellation.is_cancelled() => {}
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    return Decision::Deny;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/reply/asking.rs"]
mod tests;
