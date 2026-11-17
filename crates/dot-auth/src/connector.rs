use std::path::PathBuf;

use dot_api::{ApiKey, Cancellation, Cause, ConnectError, ConnectFailure, Connection, Connector};

use crate::anthropic::HttpKeyVerifier;
use crate::claude_code::ClaudeCodeCli;
use crate::keychain::KeychainStore;

/// [`Connector`] to Claude: an API key checked with Anthropic, or the user's plan through
/// Claude Code.
///
/// The connection lives in the macOS keychain, one at a time: connecting replaces the previous
/// one. A remembered API key counts as connected without asking Anthropic again; a remembered
/// Claude Code choice counts only while Claude Code is still signed in.
pub struct AnthropicConnector(Accounts<KeychainStore, HttpKeyVerifier, ClaudeCodeCli>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Remembered {
    ApiKey(ApiKey),
    ClaudeCode,
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
}

pub(crate) trait KeyVerifier: Send + Sync {
    fn verify(&self, key: &ApiKey) -> Verdict;
}

pub(crate) trait ClaudeCodeAccount: Send + Sync {
    fn is_signed_in(&self) -> Result<bool, ConnectError>;

    fn sign_in(&self, cancellation: &Cancellation) -> Result<(), ConnectError>;
}

struct Accounts<S, V, A> {
    store: S,
    verifier: V,
    claude_code: Option<A>,
}

impl AnthropicConnector {
    /// Creates the connector, without touching the keychain, the network or Claude Code yet.
    ///
    /// `claude_code` is the path of Claude Code's `claude` program, `None` if it is not
    /// installed (see [`crate::locate_claude_code`]).
    #[must_use]
    pub fn new(claude_code: Option<PathBuf>) -> Self {
        Self(Accounts {
            store: KeychainStore::default(),
            verifier: HttpKeyVerifier::new(),
            claude_code: claude_code.map(ClaudeCodeCli::new),
        })
    }
}

impl Connector for AnthropicConnector {
    fn current(&self) -> Result<Option<Connection>, ConnectError> {
        self.0.current()
    }

    fn connect_with_api_key(&self, key: &ApiKey) -> Result<(), ConnectError> {
        self.0.connect_with_api_key(key)
    }

    fn connect_with_claude_code(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        self.0.connect_with_claude_code(cancellation)
    }
}

impl<S: ConnectionStore, V: KeyVerifier, A: ClaudeCodeAccount> Connector for Accounts<S, V, A> {
    fn current(&self) -> Result<Option<Connection>, ConnectError> {
        match self.store.load()? {
            None => Ok(None),
            Some(Remembered::ApiKey(_)) => Ok(Some(Connection::ApiKey)),
            Some(Remembered::ClaudeCode) => {
                let Some(claude_code) = &self.claude_code else {
                    return Ok(None);
                };
                Ok(claude_code
                    .is_signed_in()?
                    .then_some(Connection::ClaudeCode))
            }
        }
    }

    fn connect_with_api_key(&self, key: &ApiKey) -> Result<(), ConnectError> {
        match self.verifier.verify(key) {
            Verdict::Accepted => self.store.save(&Remembered::ApiKey(key.clone())),
            Verdict::Rejected => Err(ConnectError::new(ConnectFailure::KeyRejected)),
            Verdict::Unreachable(cause) => Err(ConnectError::caused_by(
                ConnectFailure::AnthropicUnreachable,
                cause,
            )),
        }
    }

    fn connect_with_claude_code(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        let claude_code = self
            .claude_code
            .as_ref()
            .ok_or_else(|| ConnectError::new(ConnectFailure::ClaudeCodeMissing))?;

        if !claude_code.is_signed_in()? {
            claude_code.sign_in(cancellation)?;
            if !claude_code.is_signed_in()? {
                return Err(ConnectError::new(ConnectFailure::ClaudeCodeFailed));
            }
        }

        self.store.save(&Remembered::ClaudeCode)
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-auth/connector.rs"]
mod tests;
