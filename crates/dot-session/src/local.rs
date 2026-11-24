use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use dot_api::{Cost, Reply, Session, SessionStatus, Spending};

use crate::git;
use crate::readable::is_readable;

const UNREADABLE_CHARACTER: char = char::REPLACEMENT_CHARACTER;

/// Session in a folder of this Mac, which keeps in memory what its replies consume.
#[derive(Debug)]
pub struct LocalSession {
    folder: Option<PathBuf>,
    spending: Mutex<Spending>,
}

impl LocalSession {
    /// Creates a session in `folder`, when there is one.
    ///
    /// Nothing is read until [`Session::status`], and nothing is consumed yet.
    #[must_use]
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self {
            folder,
            spending: Mutex::default(),
        }
    }

    fn spent(&self) -> MutexGuard<'_, Spending> {
        self.spending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Session for LocalSession {
    fn status(&self) -> SessionStatus {
        let root = self.folder.as_deref().and_then(existing_folder);
        let Spending { usage, limits } = self.spending();

        SessionStatus {
            project: root.as_deref().and_then(folder_name),
            branch: root.as_deref().and_then(git::branch),
            usage,
            limits,
        }
    }

    fn spending(&self) -> Spending {
        *self.spent()
    }

    fn record(&self, reply: &Reply) {
        let mut spent = self.spent();
        let cost = spent
            .usage
            .cost
            .micro_dollars()
            .saturating_add(reply.cost.micro_dollars());
        spent.usage.cost = Cost::from_micro_dollars(cost);
        if reply.context_tokens > 0 {
            spent.usage.context_tokens = reply.context_tokens;
        }
        if let Some(base_tokens) = reply.base_tokens {
            spent.usage.base_tokens = base_tokens;
        }
        spent.limits.five_hours = reply.limits.five_hours.or(spent.limits.five_hours);
        spent.limits.seven_days = reply.limits.seven_days.or(spent.limits.seven_days);
    }

    fn count_changes(&self, added: u64, removed: u64) {
        let mut spent = self.spent();
        spent.usage.lines_added = spent.usage.lines_added.saturating_add(added);
        spent.usage.lines_removed = spent.usage.lines_removed.saturating_add(removed);
    }

    fn clear_context(&self) {
        let mut spent = self.spent();
        spent.usage.context_tokens = 0;
        spent.usage.base_tokens = 0;
    }
}

fn existing_folder(folder: &Path) -> Option<PathBuf> {
    let root = fs::canonicalize(folder).ok()?;
    root.is_dir().then_some(root)
}

fn folder_name(root: &Path) -> Option<String> {
    let name = root.file_name()?.to_string_lossy();
    Some(
        name.chars()
            .map(|character| {
                if is_readable(character) {
                    character
                } else {
                    UNREADABLE_CHARACTER
                }
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "../../../tests/dot-session/local.rs"]
mod tests;
