use std::ffi::OsStr;
use std::io::{self, Read};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Child, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dot_api::Cancellation;
use dot_process::{GRACE, Waited, command, end_groups, group_is_alive, stop, wait};

use crate::shell::output::Capture;

const BASH: &str = "/bin/bash";
const PIN: &str = "/bin/cat";
const PATH_VARIABLE: &str = "PATH";
const SCRIPT_OPTION: &str = "-c";
const SIGNALLED_BASE: i32 = 128;
const READ_CHUNK: usize = 64 * 1_024;
const SETTLE_POLL: Duration = Duration::from_millis(1);
const LIVE_EVERY: Duration = Duration::from_millis(100);
pub(crate) const PIPE_GRACE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    Exited(i32),
    TimedOut,
    Stopped,
}

pub(crate) struct Lingering {
    pub(super) group: u32,
    pin: Child,
    reader: Option<JoinHandle<()>>,
}

pub(crate) struct Ran {
    pub(crate) ending: Ending,
    pub(crate) capture: Capture,
    pub(crate) lingering: Option<Lingering>,
}

pub(crate) fn execute(
    script: &str,
    folder: Option<&Path>,
    path: Option<&OsStr>,
    (timeout, cancellation): (Duration, &Cancellation),
    on_output: &mut dyn FnMut(&[u8]),
) -> io::Result<Ran> {
    let mut bash = command(BASH);
    bash.args([SCRIPT_OPTION, script]).stdout(Stdio::piped());
    if let Some(folder) = folder {
        bash.current_dir(folder);
    }
    if let Some(path) = path {
        bash.env(PATH_VARIABLE, path);
    }
    let mut child = bash.spawn()?;
    let capture = Arc::new(Mutex::new(Capture::default()));
    let reader = child
        .stdout
        .take()
        .map(|stdout| listen(stdout, Arc::clone(&capture)));

    let ending = match run_to_the_end(&mut child, timeout, cancellation, &capture, on_output) {
        Ok(ending) => ending,
        Err(error) => {
            drop(stop(&mut child));
            return Err(error);
        }
    };
    let reader = reader.and_then(settle);
    let capture = std::mem::replace(&mut *lock(&capture), Capture::abandoned());
    Ok(Ran {
        ending,
        capture,
        lingering: Lingering::pinned(child.id(), reader),
    })
}

impl Lingering {
    fn pinned(group: u32, reader: Option<JoinHandle<()>>) -> Option<Self> {
        if !group_is_alive(group) {
            return None;
        }
        let pin = command(PIN)
            .process_group(i32::try_from(group).ok()?)
            .stdin(Stdio::piped())
            .spawn()
            .ok()?;
        Some(Self { group, pin, reader })
    }

    pub(crate) fn is_finished(&mut self) -> bool {
        !matches!(self.pin.try_wait(), Ok(None))
    }

    pub(crate) fn stop_all(lingering: Vec<Self>) {
        let groups: Vec<u32> = lingering.iter().map(|lingering| lingering.group).collect();
        if let Err(error) = end_groups(&groups, GRACE) {
            tracing::warn!(%error, "Dot could not end what Claude's commands left running");
        }
        for mut lingering in lingering {
            drop(lingering.pin.stdin.take());
            if let Err(error) = lingering.pin.wait() {
                tracing::warn!(%error, "Dot could not collect what kept a command's group");
            }
            drop(lingering.reader.and_then(settle));
        }
    }
}

fn run_to_the_end(
    child: &mut Child,
    timeout: Duration,
    cancellation: &Cancellation,
    capture: &Mutex<Capture>,
    on_output: &mut dyn FnMut(&[u8]),
) -> io::Result<Ending> {
    let deadline = Instant::now() + timeout;
    loop {
        let look = (Instant::now() + LIVE_EVERY).min(deadline);
        match wait(child, Some(look), cancellation)? {
            Waited::Exited(status) => return Ok(Ending::Exited(code(status))),
            Waited::Cancelled => return stopped(child, Ending::Stopped),
            Waited::TimedOut if Instant::now() >= deadline => {
                return stopped(child, Ending::TimedOut);
            }
            Waited::TimedOut => hand_out(capture, on_output),
        }
    }
}

fn listen(mut stdout: impl Read + Send + 'static, capture: Arc<Mutex<Capture>>) -> JoinHandle<()> {
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

fn hand_out(capture: &Mutex<Capture>, on_output: &mut dyn FnMut(&[u8])) {
    let fresh = lock(capture).take_fresh();
    if !fresh.is_empty() {
        on_output(&fresh);
    }
}

fn stopped(child: &mut Child, ending: Ending) -> io::Result<Ending> {
    stop(child)?;
    Ok(ending)
}

fn settle(reader: JoinHandle<()>) -> Option<JoinHandle<()>> {
    let until = Instant::now() + PIPE_GRACE;
    while !reader.is_finished() && Instant::now() < until {
        thread::sleep(SETTLE_POLL);
    }
    if !reader.is_finished() {
        return Some(reader);
    }
    let joined = reader.join();
    debug_assert!(joined.is_ok(), "reading a pipe does not panic");
    None
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
