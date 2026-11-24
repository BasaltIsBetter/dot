mod api_key;
mod credential;
mod secret;

use std::error::Error;
use std::fmt;

use crate::cancellation::Cancellation;
use crate::text::Text;

pub use self::api_key::{ApiKey, ApiKeyError};
pub use self::credential::{AccessToken, AccessTokenError, Credential};
pub use self::secret::Secret;

/// Boxed technical cause of a [`ConnectError`] or a [`crate::ReplyError`].
pub type Cause = Box<dyn Error + Send + Sync>;

/// Way Dot reaches Claude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    /// The user's Claude plan, signed in through Claude Code, which carries every request.
    ClaudeCode,
    /// An Anthropic Console account, signed in through Anthropic's `ant` tool and billed per
    /// use to its organization.
    AnthropicAccount,
    /// An Anthropic API key, billed per use to its Console organization.
    ApiKey,
}

/// Reason connecting to Claude failed, as told to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectFailure {
    /// Anthropic refused the API key.
    KeyRejected,
    /// Anthropic could not be reached, or answered with an error of its own.
    AnthropicUnreachable,
    /// Claude Code is not installed on this Mac.
    ClaudeCodeMissing,
    /// Signing in through Claude Code did not complete.
    ClaudeCodeFailed,
    /// Anthropic's `ant` tool is not installed on this Mac.
    AntMissing,
    /// Signing in to the Anthropic account through `ant` did not complete, or `ant` no longer
    /// hands out its access token.
    AccountFailed,
    /// Nothing connects Dot to Claude any more.
    NotConnected,
    /// The macOS keychain could not be read or written.
    StorageFailed,
    /// Dot itself failed while it connected: a bug, reported rather than left waiting.
    Crashed,
}

/// Error from a [`Connector`]: what failed, and the technical cause when there is one.
///
/// `Display` writes the identifier of the failure's [`Text`]; the cause, from a library or the
/// system, is in English.
#[derive(Debug)]
pub struct ConnectError {
    failure: ConnectFailure,
    cause: Option<Cause>,
}

/// Connects Dot to Claude and remembers how, from one launch to the next.
///
/// Every method may block on the keychain, the network, Claude Code or `ant`: call them off the
/// interface thread.
pub trait Connector: Send + Sync {
    /// Returns how Dot is connected, or `None` if nothing is connected.
    ///
    /// Checking a remembered Claude Code or account asks the program, which stops as soon as
    /// `cancellation` is raised.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectError`] if the remembered connection cannot be read or checked, or if
    /// the check is cancelled.
    fn current(&self, cancellation: &Cancellation) -> Result<Option<Connection>, ConnectError>;

    /// Checks the key with Anthropic, then keeps it in place of any previous connection.
    ///
    /// The request to Anthropic is cut as soon as `cancellation` is raised.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectError`] if Anthropic refuses the key or cannot be reached, if the
    /// check is cancelled, or if the key cannot be stored.
    fn connect_with_api_key(
        &self,
        key: &ApiKey,
        cancellation: &Cancellation,
    ) -> Result<(), ConnectError>;

    /// Signs in through Claude Code if it is not signed in yet, then keeps that choice in place
    /// of any previous connection.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectError`] if Claude Code is missing, if its sign-in fails or is
    /// cancelled through `cancellation`, or if the choice cannot be stored.
    fn connect_with_claude_code(&self, cancellation: &Cancellation) -> Result<(), ConnectError>;

    /// Signs in to an Anthropic account through `ant` if it is not signed in yet, then keeps
    /// that choice in place of any previous connection.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectError`] if `ant` is missing, if its sign-in fails or is cancelled
    /// through `cancellation`, or if the choice cannot be stored.
    fn connect_with_anthropic_account(
        &self,
        cancellation: &Cancellation,
    ) -> Result<(), ConnectError>;

    /// Returns what authenticates the next request to Claude, for the current connection.
    ///
    /// An API key is read once, then kept in memory; an account's access token is asked of
    /// `ant` each time, which renews it when it nears expiry; `ant` is stopped as soon as
    /// `cancellation` is raised.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectError`] if nothing is connected, if the key cannot be read, or if
    /// `ant` hands out no access token, in time or at all.
    fn credential(&self, cancellation: &Cancellation) -> Result<Credential, ConnectError>;

    /// Forgets the connection Dot keeps: its API key in the keychain, or its choice of Claude
    /// Code or of an Anthropic account. Claude Code and `ant` stay signed in: the next launch
    /// of Dot offers to connect again.
    ///
    /// Touches the keychain and the disk: call it off the interface thread.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectError`] if what Dot keeps cannot be removed.
    fn forget(&self) -> Result<(), ConnectError>;
}

impl ConnectFailure {
    /// Returns the text that explains the failure to the user.
    #[must_use]
    pub const fn text(self) -> Text {
        match self {
            Self::KeyRejected => Text::SignInKeyRejected,
            Self::AnthropicUnreachable => Text::SignInAnthropicUnreachable,
            Self::ClaudeCodeMissing => Text::SignInClaudeCodeMissing,
            Self::ClaudeCodeFailed => Text::SignInClaudeCodeFailed,
            Self::AntMissing => Text::SignInAntMissing,
            Self::AccountFailed => Text::SignInAccountFailed,
            Self::NotConnected => Text::SignInNotConnected,
            Self::StorageFailed => Text::SignInStorageFailed,
            Self::Crashed => Text::SignInCrashed,
        }
    }
}

impl ConnectError {
    /// Creates an error without technical cause.
    #[must_use]
    pub const fn new(failure: ConnectFailure) -> Self {
        Self {
            failure,
            cause: None,
        }
    }

    /// Creates an error with the technical cause behind it.
    #[must_use]
    pub fn caused_by(failure: ConnectFailure, cause: impl Into<Cause>) -> Self {
        Self {
            failure,
            cause: Some(cause.into()),
        }
    }

    /// Returns what failed.
    #[must_use]
    pub const fn failure(&self) -> ConnectFailure {
        self.failure
    }
}

impl fmt::Display for ConnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.failure.text().id())
    }
}

impl Error for ConnectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| -> &(dyn Error + 'static) { cause })
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/connection/mod.rs"]
mod tests;
