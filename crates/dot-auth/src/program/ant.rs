use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dot_api::{AccessToken, Cancellation, ConnectError, ConnectFailure};

use crate::connector::AnthropicAccount;
use crate::program::{Program, locate};

const PROGRAM: &str = "ant";
const HOME_DIRECTORIES: [&str; 1] = ["go/bin"];
const TOKEN_ARGUMENTS: [&str; 3] = ["auth", "print-credentials", "--access-token"];
const SIGN_IN_ARGUMENTS: [&str; 2] = ["auth", "login"];
const TOKEN_TIMEOUT: Duration = Duration::from_secs(30);
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(360);
const LONGEST_TOKEN_OUTPUT: u64 = 8_192;

pub(crate) struct AntCli(Program);

/// Returns the path of Anthropic's `ant` program, or `None` if it is not installed.
///
/// Looks in each absolute directory of `path` (the `PATH` variable), then where `ant` gets
/// installed: `~/go/bin` under `home`, `/opt/homebrew/bin` and `/usr/local/bin`. An app opened
/// from the Finder gets a short `PATH`, hence the fixed places.
#[must_use]
pub fn locate_ant(home: Option<&Path>, path: Option<&OsStr>) -> Option<PathBuf> {
    locate(PROGRAM, &HOME_DIRECTORIES, home, path, Path::is_file)
}

impl AntCli {
    pub(crate) const fn new(program: PathBuf) -> Self {
        Self(Program::new(program))
    }
}

impl AnthropicAccount for AntCli {
    fn access_token(&self, cancellation: &Cancellation) -> Result<AccessToken, ConnectError> {
        let printed = self
            .0
            .output(
                &TOKEN_ARGUMENTS,
                TOKEN_TIMEOUT,
                LONGEST_TOKEN_OUTPUT,
                cancellation,
            )
            .map_err(account_failed)?;
        AccessToken::parse(&printed).map_err(account_failed)
    }

    fn sign_in(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        self.0
            .run(&SIGN_IN_ARGUMENTS, SIGN_IN_TIMEOUT, cancellation)
            .map_err(account_failed)
    }
}

fn account_failed(cause: impl Into<dot_api::Cause>) -> ConnectError {
    ConnectError::caused_by(ConnectFailure::AccountFailed, cause)
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/program/ant.rs"]
mod tests;
