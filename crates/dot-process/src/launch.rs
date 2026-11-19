use std::ffi::OsStr;
use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

const CREDENTIAL_VARIABLES: [&str; 2] = ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"];
const OWN_PROCESS_GROUP: i32 = 0;
const KILL: &str = "/bin/kill";
const KILL_SIGNAL: &str = "-KILL";
const END_OF_OPTIONS: &str = "--";

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

/// Ends at once every process of the group `leader` leads, with `SIGKILL`.
///
/// # Errors
///
/// Returns an error if `/bin/kill` cannot run. A group already gone is no error.
pub fn kill_group(leader: u32) -> io::Result<()> {
    Command::new(KILL)
        .args([KILL_SIGNAL, END_OF_OPTIONS, &format!("-{leader}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(drop)
}

/// Ends `child` and every process of its group, then waits for `child` to be gone.
///
/// Does nothing to a child that has already ended.
///
/// # Errors
///
/// Returns an error if the child's state cannot be read or the group cannot be stopped.
pub fn stop(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    kill_group(child.id())?;
    match child.kill() {
        Err(error) if error.kind() != io::ErrorKind::InvalidInput => return Err(error),
        Ok(()) | Err(_) => {}
    }
    child.wait().map(drop)
}

#[cfg(test)]
#[path = "../../../tests/dot-process/launch.rs"]
mod tests;
