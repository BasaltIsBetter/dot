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

#[derive(Default)]
struct State {
    cancelled: AtomicBool,
    actions: Mutex<Vec<Action>>,
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
            let mut actions = self.actions();
            if self.0.cancelled.swap(true, Ordering::AcqRel) {
                return;
            }
            mem::take(&mut *actions)
        };
        for action in actions {
            action();
        }
    }

    /// Returns whether the flag has been raised.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    /// Runs `action` when the flag is raised: at once on this thread if it already is,
    /// otherwise on the thread that raises it.
    ///
    /// The action is kept until then, with what it owns: a task registers it once, before it
    /// blocks.
    pub fn on_cancel(&self, action: impl FnOnce() + Send + 'static) {
        {
            let mut actions = self.actions();
            if !self.is_cancelled() {
                actions.push(Box::new(action));
                return;
            }
        }
        action();
    }

    fn actions(&self) -> MutexGuard<'_, Vec<Action>> {
        self.0
            .actions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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

#[cfg(test)]
#[path = "../../../tests/dot-api/cancellation.rs"]
mod tests;
