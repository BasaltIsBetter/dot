use std::io::{self, Read};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{ChildStdout, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dot_api::Cancellation;
use dot_process::{Waited, command, kill_group, stop, wait};

use crate::shell::output::Capture;

const BASH: &str = "/bin/bash";
const SCRIPT_OPTION: &str = "-c";
const SIGNALLED_BASE: i32 = 128;
const READ_CHUNK: usize = 64 * 1_024;
const SETTLE_POLL: Duration = Duration::from_millis(1);
pub(crate) const PIPE_GRACE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    Exited(i32),
    TimedOut,
    Stopped,
}

pub(crate) struct Lingering {
    group: u32,
    reader: JoinHandle<()>,
}

pub(crate) struct Ran {
    pub(crate) ending: Ending,
    pub(crate) capture: Capture,
    pub(crate) lingering: Option<Lingering>,
}

pub(crate) fn execute(
    script: &str,
    folder: Option<&Path>,
    timeout: Duration,
    cancellation: &Cancellation,
) -> io::Result<Ran> {
    let mut bash = command(BASH);
    bash.args([SCRIPT_OPTION, script]).stdout(Stdio::piped());
    if let Some(folder) = folder {
        bash.current_dir(folder);
    }
    let mut child = bash.spawn()?;
    let capture = Arc::new(Mutex::new(Capture::default()));
    let reader = child
        .stdout
        .take()
        .map(|stdout| listen(stdout, Arc::clone(&capture)));

    let ending = match wait(&mut child, Some(Instant::now() + timeout), cancellation)? {
        Waited::Exited(status) => Ending::Exited(code(status)),
        Waited::TimedOut => stopped(&mut child, Ending::TimedOut)?,
        Waited::Cancelled => stopped(&mut child, Ending::Stopped)?,
    };
    let lingering = reader.and_then(|reader| settle(reader, child.id()));
    let capture = std::mem::take(&mut *lock(&capture));
    Ok(Ran {
        ending,
        capture,
        lingering,
    })
}

impl Lingering {
    pub(crate) fn is_finished(&self) -> bool {
        self.reader.is_finished()
    }

    pub(crate) fn stop(self) {
        let killed = kill_group(self.group);
        debug_assert!(killed.is_ok(), "/bin/kill is part of macOS");
        drop(settle(self.reader, self.group));
    }
}

fn listen(mut stdout: ChildStdout, capture: Arc<Mutex<Capture>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = vec![0; READ_CHUNK];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => return,
                Ok(read) => lock(&capture).push(&buffer[..read]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return,
            }
        }
    })
}

fn stopped(child: &mut std::process::Child, ending: Ending) -> io::Result<Ending> {
    stop(child)?;
    Ok(ending)
}

fn settle(reader: JoinHandle<()>, group: u32) -> Option<Lingering> {
    let until = Instant::now() + PIPE_GRACE;
    while !reader.is_finished() && Instant::now() < until {
        thread::sleep(SETTLE_POLL);
    }
    if reader.is_finished() {
        let joined = reader.join();
        debug_assert!(joined.is_ok(), "reading a pipe does not panic");
        return None;
    }
    Some(Lingering { group, reader })
}

fn code(status: ExitStatus) -> i32 {
    status
        .code()
        .or_else(|| status.signal().map(|signal| SIGNALLED_BASE + signal))
        .unwrap_or(SIGNALLED_BASE)
}

fn lock(capture: &Mutex<Capture>) -> MutexGuard<'_, Capture> {
    capture.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/shell/process.rs"]
mod tests;
