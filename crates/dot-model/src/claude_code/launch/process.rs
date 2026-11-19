use std::io;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::Cancellation;

use crate::claude_code::launch::command::kill_group;

const FIRST_POLL: Duration = Duration::from_millis(1);
const LONGEST_POLL: Duration = Duration::from_millis(50);

pub(crate) struct Running {
    child: Arc<Mutex<Child>>,
}

pub(crate) struct Pipes {
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) stdout: Option<ChildStdout>,
}

impl Running {
    pub(crate) fn start(
        mut command: Command,
        cancellation: &Cancellation,
    ) -> io::Result<(Self, Pipes)> {
        let mut child = command.spawn()?;
        let pipes = Pipes {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
        };
        let child = Arc::new(Mutex::new(child));

        let stopped_on_cancel = Arc::clone(&child);
        cancellation.on_cancel(move || {
            let stopped = stop(&mut lock(&stopped_on_cancel));
            debug_assert!(stopped.is_ok(), "Dot can always stop its own Claude Code");
        });
        Ok((Self { child }, pipes))
    }

    pub(crate) fn finish(&self, grace: Duration) -> io::Result<ExitStatus> {
        let started = Instant::now();
        let mut pause = FIRST_POLL;
        loop {
            if let Some(status) = lock(&self.child).try_wait()? {
                return Ok(status);
            }
            if started.elapsed() >= grace {
                stop(&mut lock(&self.child))?;
            }
            thread::sleep(pause);
            pause = longer(pause);
        }
    }
}

fn longer(pause: Duration) -> Duration {
    (pause * 2).min(LONGEST_POLL)
}

fn stop(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    kill_group(child.id())?;
    child.kill()
}

fn lock(child: &Mutex<Child>) -> MutexGuard<'_, Child> {
    child.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/process.rs"]
mod tests;
