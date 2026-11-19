mod prompt;

use std::error::Error;
use std::fmt;

use crate::cancellation::Cancellation;
use crate::connection::{Cause, Credential};
use crate::session::{Cost, Limits};
use crate::text::Text;
use crate::tool::{Permissions, ToolEnd, ToolUse};

pub use self::prompt::{Image, ImageError, ImageFormat, Prompt};

/// Piece of Claude's reply, handed over as soon as it arrives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyEvent {
    /// A piece of the summary of Claude's thinking, written before or between the answers.
    Thinking(String),
    /// A piece of the answer.
    Text(String),
    /// Claude calls a tool, which may ask the user first.
    ToolStarted {
        /// Identifier of the call, unique within the conversation.
        id: String,
        /// What the tool does, as shown.
        tool: ToolUse,
    },
    /// A tool call ended.
    ToolEnded {
        /// Identifier of the call, as when it started.
        id: String,
        /// How it ended.
        end: ToolEnd,
    },
}

/// What a reply Claude finished has consumed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    /// Tokens in Claude's context window once the reply is written.
    pub context_tokens: u64,
    /// What the reply cost: billed with an API key or an account, an estimate on a Claude plan.
    pub cost: Cost,
    /// Share of the Claude plan's usage limits spent, when Claude reports it with the reply.
    pub limits: Limits,
}

/// Reason a reply did not end as Claude finished it, as told to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyFailure {
    /// The user stopped the reply.
    Stopped,
    /// Anthropic refused the credential.
    Unauthorized,
    /// Anthropic refused the request itself, for instance a conversation too long for the
    /// model.
    Rejected,
    /// A usage or rate limit is reached.
    RateLimited,
    /// Claude is overloaded.
    Overloaded,
    /// Anthropic could not be reached, or failed on its side.
    Unreachable,
    /// Claude declined to answer.
    Declined,
    /// Claude Code could not run or answered with an error of its own.
    ClaudeCodeFailed,
}

/// Error from a [`Chat`]: what failed, and the technical cause when there is one.
///
/// `Display` writes the identifier of the failure's [`Text`]; the cause, from a library, Claude
/// Code or Anthropic, is in English.
#[derive(Debug)]
pub struct ReplyError {
    failure: ReplyFailure,
    cause: Option<Cause>,
}

/// Conversation of one window with Claude, carried on from one prompt to the next.
pub trait Chat: Send + Sync {
    /// Sends `prompt`, its text and its images, to Claude as the next message of the
    /// conversation, then hands the reply to `on_event` piece by piece, until Claude has
    /// finished.
    ///
    /// Claude may use tools along the way, each one announced and ended through `on_event`.
    /// Whatever writes, runs a command or reaches outside the session's folder first asks
    /// `permissions`, on this thread, and waits for the answer.
    ///
    /// Blocks until the reply ends, on the network or on Claude Code: call it off the interface
    /// thread, one reply at a time. `credential` says how to reach Claude. Raising
    /// `cancellation` ends the reply at once, a running tool included; what Claude wrote until
    /// then stays in the conversation.
    ///
    /// # Errors
    ///
    /// Returns a [`ReplyError`] if Claude cannot be reached, refuses the request or the
    /// credential, declines to answer, or if the reply is stopped through `cancellation`.
    fn reply(
        &self,
        prompt: &Prompt,
        credential: &Credential,
        cancellation: &Cancellation,
        permissions: &dyn Permissions,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError>;
}

impl ReplyFailure {
    /// Returns the text that explains the failure to the user.
    #[must_use]
    pub const fn text(self) -> Text {
        match self {
            Self::Stopped => Text::ReplyStopped,
            Self::Unauthorized => Text::ReplyUnauthorized,
            Self::Rejected => Text::ReplyRejected,
            Self::RateLimited => Text::ReplyRateLimited,
            Self::Overloaded => Text::ReplyOverloaded,
            Self::Unreachable => Text::ReplyUnreachable,
            Self::Declined => Text::ReplyDeclined,
            Self::ClaudeCodeFailed => Text::ReplyClaudeCodeFailed,
        }
    }
}

impl ReplyError {
    /// Creates an error without technical cause.
    #[must_use]
    pub const fn new(failure: ReplyFailure) -> Self {
        Self {
            failure,
            cause: None,
        }
    }

    /// Creates an error with the technical cause behind it.
    #[must_use]
    pub fn caused_by(failure: ReplyFailure, cause: impl Into<Cause>) -> Self {
        Self {
            failure,
            cause: Some(cause.into()),
        }
    }

    /// Returns what failed.
    #[must_use]
    pub const fn failure(&self) -> ReplyFailure {
        self.failure
    }
}

impl fmt::Display for ReplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.failure.text().id())
    }
}

impl Error for ReplyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| -> &(dyn Error + 'static) { cause })
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/chat/mod.rs"]
mod tests;
