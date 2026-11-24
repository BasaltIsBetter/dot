use std::path::{Path, PathBuf};

use dot_api::{AccessToken, ApiKey, Cancellation, Cause, ConnectError, Connector};

use crate::accounts::Accounts;
use crate::anthropic::HttpKeyVerifier;
use crate::program::{AntCli, ClaudeCodeCli};
use crate::store::{ChoiceFile, KeychainStore, SplitStore};

const CHOICE_FILE: &str = "Library/Application Support/Dot/connection";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Remembered {
    ApiKey(ApiKey),
    ClaudeCode,
    AnthropicAccount,
}

#[derive(Debug)]
pub(crate) enum Verdict {
    Accepted,
    Rejected,
    Unreachable(Cause),
}

pub(crate) trait ConnectionStore: Send + Sync {
    fn load(&self) -> Result<Option<Remembered>, ConnectError>;

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError>;

    fn forget(&self) -> Result<(), ConnectError>;
}

pub(crate) trait KeyVerifier: Send + Sync {
    fn verify(&self, key: &ApiKey, cancellation: &Cancellation) -> Verdict;
}

pub(crate) trait ClaudeCodeAccount: Send + Sync {
    fn is_signed_in(&self, cancellation: &Cancellation) -> Result<bool, ConnectError>;

    fn sign_in(&self, cancellation: &Cancellation) -> Result<(), ConnectError>;
}

pub(crate) trait AnthropicAccount: Send + Sync {
    fn access_token(&self, cancellation: &Cancellation) -> Result<AccessToken, ConnectError>;

    fn sign_in(&self, cancellation: &Cancellation) -> Result<(), ConnectError>;
}

/// Returns the [`Connector`] to Claude: the user's plan through Claude Code, an Anthropic account
/// through `ant`, or an API key checked with Anthropic.
///
/// One connection at a time: connecting replaces the previous one. An API key lives in the macOS
/// keychain; the choice of Claude Code or of an account, which holds no secret, lives in
/// `~/Library/Application Support/Dot/connection`. A remembered API key counts as connected
/// without asking Anthropic again; a remembered Claude Code or account counts only while it is
/// still signed in.
///
/// Nothing is touched yet: neither the disk, the keychain, the network, Claude Code nor `ant`.
/// `claude_code` is the path of Claude Code's `claude` program and `ant` the path of Anthropic's
/// `ant`, `None` when not installed (see [`crate::locate_claude_code`] and
/// [`crate::locate_ant`]). `home` is the user's home folder; without it, the choice of Claude
/// Code or of an account is kept in the keychain too.
#[must_use]
pub fn anthropic_connector(
    claude_code: Option<PathBuf>,
    ant: Option<PathBuf>,
    home: Option<&Path>,
) -> impl Connector + use<> {
    let choice = home.map(|home| ChoiceFile::new(home.join(CHOICE_FILE)));
    Accounts::new(
        SplitStore::new(choice, KeychainStore::default()),
        HttpKeyVerifier::new(),
        claude_code.map(ClaudeCodeCli::new),
        ant.map(AntCli::new),
    )
}

#[cfg(test)]
#[path = "../../../tests/dot-auth/connector.rs"]
mod tests;
