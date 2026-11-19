use std::fmt;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

type Action = Box<dyn FnOnce() + Send>;

/// Shared flag that asks a running task to stop.
///
/// Clones share the same flag: the interface keeps one and hands another to the task. A task
/// that blocks where it cannot look at the flag, on a socket or a child process, registers with
/// [`Cancellation::on_cancel`] what wakes it up, such as closing its connection.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<State>);

/// Registration of an action with [`Cancellation::on_cancel`], undone when the guard drops.
///
/// Keep the guard for as long as the task blocks: dropping it removes the action, with what it
/// owns, from the flag, so that a flag kept after the task ends holds nothing of it.
#[must_use = "dropping the guard removes the action at once"]
pub struct CancelGuard {
    state: Arc<State>,
    key: Option<u64>,
}

#[derive(Default)]
struct State {
    cancelled: AtomicBool,
    registered: Mutex<Registered>,
}

#[derive(Default)]
struct Registered {
    next_key: u64,
    actions: Vec<(u64, Action)>,
}

impl Cancellation {
    /// Creates a flag that is not raised.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Raises the flag, for every clone, then runs the registered actions on this thread.
    ///
    /// Raising it again does nothing: each action runs once.
    pub fn cancel(&self) {
        let actions = {
            let mut registered = self.0.registered();
            if self.0.cancelled.swap(true, Ordering::AcqRel) {
                return;
            }
            mem::take(&mut registered.actions)
        };
        for (_, action) in actions {
            action();
        }
    }

    /// Returns whether the flag has been raised.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    /// Runs `action` when the flag is raised: at once on this thread if it already is,
    /// otherwise on the thread that raises it, as long as the returned guard lives.
    ///
    /// The action is kept with what it owns until it runs or the guard drops: a task registers
    /// it before it blocks, and drops the guard once it no longer blocks.
    pub fn on_cancel(&self, action: impl FnOnce() + Send + 'static) -> CancelGuard {
        let mut registered = self.0.registered();
        if self.is_cancelled() {
            drop(registered);
            action();
            return self.guard(None);
        }

        let key = registered.next_key;
        registered.next_key += 1;
        registered.actions.push((key, Box::new(action)));
        drop(registered);
        self.guard(Some(key))
    }

    fn guard(&self, key: Option<u64>) -> CancelGuard {
        CancelGuard {
            state: Arc::clone(&self.0),
            key,
        }
    }
}

impl State {
    fn registered(&self) -> MutexGuard<'_, Registered> {
        self.registered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        let Some(key) = self.key else {
            return;
        };
        self.state
            .registered()
            .actions
            .retain(|(registered, _)| *registered != key);
    }
}

impl fmt::Debug for Cancellation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Cancellation")
            .field(&self.is_cancelled())
            .finish()
    }
}

impl fmt::Debug for CancelGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CancelGuard")
            .field("registered", &self.key.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-api/cancellation.rs"]
mod tests;
