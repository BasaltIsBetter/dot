use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dot_api::{Cancellation, ConnectError, ConnectFailure};

use crate::connector::ClaudeCodeAccount;
use crate::program::{Program, RunError, locate};

const PROGRAM: &str = "claude";
const HOME_DIRECTORIES: [&str; 2] = [".local/bin", ".claude/local"];
const STATUS_ARGUMENTS: [&str; 2] = ["auth", "status"];
const SIGN_IN_ARGUMENTS: [&str; 3] = ["auth", "login", "--claudeai"];
const STATUS_TIMEOUT: Duration = Duration::from_secs(15);
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(600);

pub(crate) struct ClaudeCodeCli(Program);

/// Returns the path of Claude Code's `claude` program, or `None` if it is not installed.
///
/// Looks in each absolute directory of `path` (the `PATH` variable), then where Claude Code's
/// installers put it: `~/.local/bin`, `~/.claude/local` under `home`, `/opt/homebrew/bin` and
/// `/usr/local/bin`. An app opened from the Finder gets a short `PATH`, hence the fixed places.
#[must_use]
pub fn locate_claude_code(home: Option<&Path>, path: Option<&OsStr>) -> Option<PathBuf> {
    locate(PROGRAM, &HOME_DIRECTORIES, home, path, Path::is_file)
}

impl ClaudeCodeCli {
    pub(crate) const fn new(program: PathBuf) -> Self {
        Self(Program::new(program))
    }
}

impl ClaudeCodeAccount for ClaudeCodeCli {
    fn is_signed_in(&self, cancellation: &Cancellation) -> Result<bool, ConnectError> {
        match self.0.run(&STATUS_ARGUMENTS, STATUS_TIMEOUT, cancellation) {
            Ok(()) => Ok(true),
            Err(RunError::Failed(_)) => Ok(false),
            Err(error) => Err(sign_in_failed(error)),
        }
    }

    fn sign_in(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        self.0
            .run(&SIGN_IN_ARGUMENTS, SIGN_IN_TIMEOUT, cancellation)
            .map_err(sign_in_failed)
    }
}

fn sign_in_failed(error: RunError) -> ConnectError {
    ConnectError::caused_by(ConnectFailure::ClaudeCodeFailed, error)
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/program/claude_code.rs"]
mod tests;
