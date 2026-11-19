use std::io;
use std::process::{Child, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::Cancellation;

const FIRST_POLL: Duration = Duration::from_millis(1);
const LONGEST_POLL: Duration = Duration::from_millis(50);

/// How [`wait`] ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waited {
    /// The program ended by itself.
    Exited(ExitStatus),
    /// The deadline came first; the program still runs.
    TimedOut,
    /// The task was cancelled first; the program still runs.
    Cancelled,
}

/// Waits for `child` to end, until `deadline` when there is one, or until `cancellation` is
/// raised.
///
/// Looks at the child after 1 ms, then twice as late each time, 50 ms apart at most: a quick
/// program is seen ending at once, a long one costs almost nothing. Stops nothing itself: a
/// program still running is the caller's to [`stop`](crate::stop).
///
/// # Errors
///
/// Returns an error if the child's state cannot be read.
pub fn wait(
    child: &mut Child,
    deadline: Option<Instant>,
    cancellation: &Cancellation,
) -> io::Result<Waited> {
    let mut pause = FIRST_POLL;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Waited::Exited(status));
        }
        if cancellation.is_cancelled() {
            return Ok(Waited::Cancelled);
        }
        let now = Instant::now();
        if deadline.is_some_and(|deadline| now >= deadline) {
            return Ok(Waited::TimedOut);
        }
        let left = deadline.map_or(pause, |deadline| deadline - now);
        thread::sleep(pause.min(left));
        pause = (pause * 2).min(LONGEST_POLL);
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-process/wait.rs"]
mod tests;
