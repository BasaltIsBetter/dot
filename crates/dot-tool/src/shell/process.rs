use std::io::{self, Read};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dot_api::Cancellation;

use crate::shell::output::Capture;

const BASH: &str = "/bin/bash";
const SCRIPT_OPTION: &str = "-c";
const CREDENTIAL_VARIABLES: [&str; 2] = ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"];
const OWN_PROCESS_GROUP: i32 = 0;
const KILL: &str = "/bin/kill";
const KILL_SIGNAL: &str = "-KILL";
const END_OF_OPTIONS: &str = "--";
const SIGNALLED_BASE: i32 = 128;
const READ_CHUNK: usize = 64 * 1_024;
const FIRST_POLL: Duration = Duration::from_millis(1);
const LONGEST_POLL: Duration = Duration::from_millis(50);
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
    let mut command = Command::new(BASH);
    command
        .args([SCRIPT_OPTION, script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(OWN_PROCESS_GROUP);
    for variable in CREDENTIAL_VARIABLES {
        command.env_remove(variable);
    }
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    let mut child = command.spawn()?;
    let capture = Arc::new(Mutex::new(Capture::default()));
    let reader = child
        .stdout
        .take()
        .map(|stdout| listen(stdout, Arc::clone(&capture)));

    let ending = wait(&mut child, timeout, cancellation)?;
    let lingering = reader.and_then(|reader| settle(reader, child.id()));
    let capture = std::mem::take(&mut *lock(&capture));
    Ok(Ran {
        ending,
        capture,
        lingering,
    })
}

pub(crate) fn kill_group(group: u32) {
    let killed = Command::new(KILL)
        .args([KILL_SIGNAL, END_OF_OPTIONS, &format!("-{group}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    debug_assert!(killed.is_ok(), "/bin/kill is part of macOS");
}

impl Lingering {
    pub(crate) fn is_finished(&self) -> bool {
        self.reader.is_finished()
    }

    pub(crate) fn stop(self) {
        kill_group(self.group);
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

fn wait(child: &mut Child, timeout: Duration, cancellation: &Cancellation) -> io::Result<Ending> {
    let deadline = Instant::now() + timeout;
    let mut pause = FIRST_POLL;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Ending::Exited(code(status)));
        }
        let ending = if cancellation.is_cancelled() {
            Ending::Stopped
        } else if Instant::now() >= deadline {
            Ending::TimedOut
        } else {
            thread::sleep(pause);
            pause = (pause * 2).min(LONGEST_POLL);
            continue;
        };
        kill_group(child.id());
        child.kill().or_else(|error| match error.kind() {
            io::ErrorKind::InvalidInput => Ok(()),
            _ => Err(error),
        })?;
        child.wait()?;
        return Ok(ending);
    }
}

fn settle(reader: JoinHandle<()>, group: u32) -> Option<Lingering> {
    let until = Instant::now() + PIPE_GRACE;
    while !reader.is_finished() && Instant::now() < until {
        thread::sleep(FIRST_POLL);
    }
    if reader.is_finished() {
        if let Err(panic) = reader.join() {
            std::panic::resume_unwind(panic);
        }
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
