use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::{Cancellation, ConnectError, ConnectFailure};

use crate::connector::ClaudeCodeAccount;

const PROGRAM: &str = "claude";
const HOME_DIRECTORIES: [&str; 2] = [".local/bin", ".claude/local"];
const SYSTEM_DIRECTORIES: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];
const STATUS_ARGUMENTS: [&str; 2] = ["auth", "status"];
const SIGN_IN_ARGUMENTS: [&str; 3] = ["auth", "login", "--claudeai"];
const STATUS_TIMEOUT: Duration = Duration::from_secs(15);
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(600);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct ClaudeCodeCli {
    program: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum RunError {
    #[error("could not run Claude Code")]
    Spawn(#[source] io::Error),
    #[error("lost track of Claude Code")]
    Wait(#[source] io::Error),
    #[error("Claude Code did not finish within {} s", .0.as_secs())]
    TimedOut(Duration),
    #[error("the sign-in was cancelled")]
    Cancelled,
    #[error("Claude Code stopped with {0}")]
    Failed(ExitStatus),
}

/// Returns the path of Claude Code's `claude` program, or `None` if it is not installed.
///
/// Looks in each absolute directory of `path` (the `PATH` variable), then where Claude Code's
/// installers put it: `~/.local/bin`, `~/.claude/local` under `home`, `/opt/homebrew/bin` and
/// `/usr/local/bin`. An app opened from the Finder gets a short `PATH`, hence the fixed places.
#[must_use]
pub fn locate_claude_code(home: Option<&Path>, path: Option<&OsStr>) -> Option<PathBuf> {
    locate(home, path, Path::is_file)
}

impl ClaudeCodeCli {
    pub(crate) fn new(program: PathBuf) -> Self {
        Self { program }
    }

    fn run(
        &self,
        arguments: &[&str],
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<ExitStatus, RunError> {
        let child = Command::new(&self.program)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(RunError::Spawn)?;
        wait(child, timeout, cancellation)
    }
}

impl ClaudeCodeAccount for ClaudeCodeCli {
    fn is_signed_in(&self) -> Result<bool, ConnectError> {
        self.run(&STATUS_ARGUMENTS, STATUS_TIMEOUT, &Cancellation::new())
            .map(|status| status.success())
            .map_err(sign_in_failed)
    }

    fn sign_in(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        let status = self
            .run(&SIGN_IN_ARGUMENTS, SIGN_IN_TIMEOUT, cancellation)
            .map_err(sign_in_failed)?;
        if !status.success() {
            return Err(sign_in_failed(RunError::Failed(status)));
        }
        Ok(())
    }
}

fn locate(
    home: Option<&Path>,
    path: Option<&OsStr>,
    is_file: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let searched = path.into_iter().flat_map(std::env::split_paths);
    let installed = home
        .into_iter()
        .flat_map(|home| HOME_DIRECTORIES.map(|directory| home.join(directory)));
    let system = SYSTEM_DIRECTORIES.map(PathBuf::from);

    searched
        .chain(installed)
        .chain(system)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(PROGRAM))
        .find(|candidate| is_file(candidate))
}

fn wait(
    mut child: Child,
    timeout: Duration,
    cancellation: &Cancellation,
) -> Result<ExitStatus, RunError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(RunError::Wait)? {
            return Ok(status);
        }
        if cancellation.is_cancelled() {
            stop(&mut child)?;
            return Err(RunError::Cancelled);
        }
        if started.elapsed() >= timeout {
            stop(&mut child)?;
            return Err(RunError::TimedOut(timeout));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn stop(child: &mut Child) -> Result<(), RunError> {
    child.kill().map_err(RunError::Wait)?;
    child.wait().map(drop).map_err(RunError::Wait)
}

fn sign_in_failed(error: RunError) -> ConnectError {
    ConnectError::caused_by(ConnectFailure::ClaudeCodeFailed, error)
}

#[cfg(test)]
#[path = "../../../tests/dot-auth/claude_code.rs"]
mod tests;
