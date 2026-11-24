use std::io::{self, BufWriter, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dot_api::{CancelGuard, Cancellation, Prompt};
use serde::Serialize;

use crate::claude_code::NEW_LINE;
use crate::content::UserMessage;

pub(crate) const CANCEL_GRACE: Duration = Duration::from_secs(2);
const FIRST_POLL: Duration = Duration::from_millis(1);
const LONGEST_POLL: Duration = Duration::from_millis(50);
const WATCH: Duration = Duration::from_millis(50);
const MOST_SAID: usize = 4 * 1_024;
const SAID_CHUNK: usize = 1_024;
const SAID_GRACE: Duration = Duration::from_millis(100);

pub(crate) struct Running {
    child: Mutex<Child>,
    leader: u32,
    said: Arc<Mutex<Vec<u8>>>,
    listener: Mutex<Option<JoinHandle<()>>>,
    _asked_to_end_on_cancel: CancelGuard,
}

#[derive(Serialize)]
struct InputLine<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    message: UserMessage<'a>,
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
        let said = Arc::new(Mutex::new(Vec::new()));
        let listener = child.stderr.take().map(|stderr| {
            let kept = Arc::clone(&said);
            thread::spawn(move || keep_the_end(stderr, &kept))
        });
        let leader = child.id();
        let asked = cancellation.on_cancel(move || ask_to_end(leader));
        Ok((
            Self {
                child: Mutex::new(child),
                leader,
                said,
                listener: Mutex::new(listener),
                _asked_to_end_on_cancel: asked,
            },
            pipes,
        ))
    }

    pub(crate) fn end_when_cancelled(
        &self,
        cancellation: &Cancellation,
        finished: &AtomicBool,
        grace: Duration,
    ) {
        let mut asked_at = None;
        while !finished.load(Ordering::Acquire) {
            if cancellation.is_cancelled() {
                let asked = *asked_at.get_or_insert_with(Instant::now);
                if self.has_ended() {
                    self.end_what_it_left();
                    return;
                }
                if asked.elapsed() >= grace {
                    self.kill();
                    self.end_what_it_left();
                    return;
                }
            }
            thread::sleep(WATCH);
        }
    }

    fn has_ended(&self) -> bool {
        matches!(locked(&self.child).try_wait(), Ok(Some(_)))
    }

    pub(crate) fn finish(&self, grace: Duration) -> io::Result<ExitStatus> {
        let started = Instant::now();
        let mut pause = FIRST_POLL;
        let status = loop {
            if let Some(status) = locked(&self.child).try_wait()? {
                break status;
            }
            if started.elapsed() >= grace {
                let mut child = locked(&self.child);
                dot_process::stop(&mut child)?;
                break child.wait()?;
            }
            thread::sleep(pause);
            pause = longer(pause);
        };
        self.end_what_it_left();
        self.stop_listening();
        Ok(status)
    }

    pub(crate) fn said(&self) -> String {
        let said = locked(&self.said);
        String::from_utf8_lossy(&said).into_owned()
    }

    fn stop_listening(&self) {
        let listener = locked(&self.listener).take();
        let Some(listener) = listener else {
            return;
        };
        let until = Instant::now() + SAID_GRACE;
        while !listener.is_finished() && Instant::now() < until {
            thread::sleep(FIRST_POLL);
        }
        if listener.is_finished() {
            let joined = listener.join();
            debug_assert!(joined.is_ok(), "reading a pipe does not panic");
        }
    }

    fn kill(&self) {
        let mut child = locked(&self.child);
        let killed = dot_process::kill_group(self.leader).and_then(|()| child.kill());
        if let Err(error) = killed {
            tracing::warn!(%error, "Dot could not stop Claude Code");
        }
    }

    fn end_what_it_left(&self) {
        if let Err(error) = dot_process::end_groups(&[self.leader], dot_process::GRACE) {
            tracing::warn!(%error, "Dot could not end what Claude Code left running");
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let child = self.child.get_mut().unwrap_or_else(PoisonError::into_inner);
        if let Err(error) = dot_process::stop(child) {
            tracing::warn!(%error, "Dot could not stop Claude Code");
        }
    }
}

fn keep_the_end(mut stderr: ChildStderr, said: &Mutex<Vec<u8>>) {
    let mut chunk = [0; SAID_CHUNK];
    loop {
        match stderr.read(&mut chunk) {
            Ok(0) => return,
            Ok(read) => {
                let mut kept = locked(said);
                kept.extend_from_slice(&chunk[..read]);
                let stale = kept.len().saturating_sub(MOST_SAID);
                kept.drain(..stale);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn ask_to_end(leader: u32) {
    if let Err(error) = dot_process::terminate_group(leader) {
        tracing::warn!(%error, "Dot could not ask Claude Code to stop");
    }
}

fn longer(pause: Duration) -> Duration {
    (pause * 2).min(LONGEST_POLL)
}

pub(crate) fn hand_over(stdin: Option<&mut ChildStdin>, prompt: &Prompt) -> io::Result<()> {
    let Some(stdin) = stdin else {
        return Ok(());
    };
    let line = InputLine {
        kind: "user",
        message: UserMessage::of(prompt),
    };
    let mut writer = BufWriter::new(stdin);
    serde_json::to_writer(&mut writer, &line)?;
    writer.write_all(NEW_LINE)?;
    writer.flush()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/process.rs"]
mod tests;
