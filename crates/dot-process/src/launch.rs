use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::io::Errno;
use rustix::process::{Pid, Signal, getpgid, getpgrp, kill_process_group, test_kill_process_group};

#[cfg(target_os = "linux")]
use crate::ended::is_more_than_ended_processes;

const CREDENTIAL_VARIABLES: [&str; 2] = ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"];
const OWN_PROCESS_GROUP: i32 = 0;
const FIRST_ORDINARY_PROCESS: u32 = 2;
const LOOK_AGAIN: Duration = Duration::from_millis(10);
const ASK_AGAIN: Duration = Duration::from_millis(100);
const KILL_ROUNDS: usize = 10;
const NOT_A_GROUP_OF_DOT: &str = "no program of Dot leads this process group";

/// How long a process group asked to end is given to do so before it is killed: half a second.
///
/// Long enough for a program to clean up after itself (`git` removes its `index.lock`), short
/// enough for a stop to feel immediate.
pub const GRACE: Duration = Duration::from_millis(500);

/// Returns the command that starts `program`, ready for its arguments.
///
/// The program reads nothing and writes nowhere until the caller pipes it, does not see
/// `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN` from Dot's own environment, and leads a process
/// group of its own, which [`stop`] ends whole.
///
/// # Examples
///
/// ```
/// let status = dot_process::command("/bin/echo").arg("hello").status().unwrap();
/// assert!(status.success());
/// ```
#[must_use]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    for variable in CREDENTIAL_VARIABLES {
        command.env_remove(variable);
    }
    command
        .process_group(OWN_PROCESS_GROUP)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Asks every process of the group `leader` leads to end, with `SIGTERM`, without waiting.
///
/// Makes no system call that can block and starts no program: it may be called from the thread
/// of the interface.
///
/// # Errors
///
/// Returns an error for a group no program of Dot may lead: 0, 1 or the caller's own group. A
/// group already gone, or in which nothing is left that the user may signal (only processes
/// waiting to be collected), is no error.
pub fn terminate_group(leader: u32) -> io::Result<()> {
    signal_group(leader, Signal::TERM)
}

/// Ends at once every process of the group `leader` leads, with `SIGKILL`.
///
/// # Errors
///
/// Returns an error for a group no program of Dot may lead: 0, 1 or the caller's own group. A
/// group already gone, or in which nothing is left that the user may signal (only processes
/// waiting to be collected), is no error.
pub fn kill_group(leader: u32) -> io::Result<()> {
    signal_group(leader, Signal::KILL)
}

/// Returns whether a process of the group `leader` leads still runs and may be signalled.
///
/// A group no program of Dot may lead is never alive, nor is one of which only processes that
/// have ended and wait to be collected are left.
#[must_use]
pub fn group_is_alive(leader: u32) -> bool {
    group(leader).is_ok_and(|group| {
        test_kill_process_group(group).is_ok() && is_more_than_ended_processes(group)
    })
}

/// Returns the process group the caller belongs to.
#[must_use]
pub fn own_group() -> u32 {
    getpgrp().as_raw_pid().unsigned_abs()
}

/// Returns the group `process` leads, when it leads one that is not the caller's.
///
/// What another program says of itself is checked this way before its group is ever signalled.
#[must_use]
pub fn group_led_by(process: u32) -> Option<u32> {
    let pid = group(process).ok()?;
    let led = getpgid(Some(pid)).ok()?;
    (led == pid).then_some(process)
}

/// Ends every group of `leaders`: asks them all to end, gives them `grace` to do so, then kills
/// what is left.
///
/// The groups are asked together and waited for together, so ending many costs one `grace` at
/// most. They are asked again every tenth of a second, and killed again until they are gone: a
/// process being started while a group is signalled misses the signal.
///
/// # Errors
///
/// Returns the first error met; every group is still asked to end and killed.
pub fn end_groups(leaders: &[u32], grace: Duration) -> io::Result<()> {
    let mut first_error = FirstError::default();
    let ask = || {
        for &leader in leaders {
            first_error.keep(terminate_group(leader));
        }
        Ok(())
    };
    ask_to_end(grace, ask, || Ok(!any_alive(leaders)))?;

    kill_what_is_left(leaders, &mut first_error);
    first_error.into_result()
}

/// Ends `child` and every process of its group, then waits for `child` to be gone.
///
/// The group is first asked to end (`SIGTERM`), again every tenth of a second, and given
/// [`GRACE`] to do so; what is left then is killed. Returns as soon as the child and its group
/// are gone. Does nothing to a child that has already ended.
///
/// # Errors
///
/// Returns an error if the child's state cannot be read or its group cannot be signalled.
pub fn stop(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    let leader = child.id();
    let ended = ask_to_end(
        GRACE,
        || terminate_group(leader),
        || Ok(child.try_wait()?.is_some() && !group_is_alive(leader)),
    )?;
    if ended {
        return Ok(());
    }

    kill_group(leader)?;
    child.kill()?;
    child.wait()?;
    let mut first_error = FirstError::default();
    kill_what_is_left(&[leader], &mut first_error);
    first_error.into_result()
}

#[derive(Default)]
struct FirstError(Option<io::Error>);

impl FirstError {
    fn keep(&mut self, result: io::Result<()>) {
        if let Err(error) = result {
            self.0.get_or_insert(error);
        }
    }

    fn into_result(self) -> io::Result<()> {
        self.0.map_or(Ok(()), Err)
    }
}

fn ask_to_end(
    grace: Duration,
    mut ask: impl FnMut() -> io::Result<()>,
    mut ended: impl FnMut() -> io::Result<bool>,
) -> io::Result<bool> {
    ask()?;
    let mut asked_at = Instant::now();
    let deadline = asked_at + grace;
    while Instant::now() < deadline {
        if ended()? {
            return Ok(true);
        }
        if asked_at.elapsed() >= ASK_AGAIN {
            ask()?;
            asked_at = Instant::now();
        }
        thread::sleep(LOOK_AGAIN);
    }
    Ok(false)
}

fn any_alive(leaders: &[u32]) -> bool {
    leaders.iter().any(|&leader| group_is_alive(leader))
}

fn kill_what_is_left(leaders: &[u32], first_error: &mut FirstError) {
    for _ in 0..KILL_ROUNDS {
        if !any_alive(leaders) {
            return;
        }
        for &leader in leaders {
            first_error.keep(kill_group(leader));
        }
        thread::sleep(LOOK_AGAIN);
    }
}

/// macOS does not answer for a group of which every process has ended; Linux does, until they are
/// collected.
#[cfg(not(target_os = "linux"))]
const fn is_more_than_ended_processes(_group: Pid) -> bool {
    true
}

fn signal_group(leader: u32, signal: Signal) -> io::Result<()> {
    match kill_process_group(group(leader)?, signal) {
        Ok(()) => Ok(()),
        Err(error) if error == Errno::SRCH || error == Errno::PERM => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn group(leader: u32) -> io::Result<Pid> {
    i32::try_from(leader)
        .ok()
        .filter(|_| leader >= FIRST_ORDINARY_PROCESS)
        .and_then(Pid::from_raw)
        .filter(|&pid| pid != getpgrp())
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, NOT_A_GROUP_OF_DOT))
}

#[cfg(test)]
#[path = "../../../tests/dot-process/launch.rs"]
mod tests;
