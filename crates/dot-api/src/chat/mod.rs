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

/// What a reply has consumed, whether Claude finished it or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    /// Tokens in Claude's context window once the reply is written; zero when the reply never
    /// reached Claude.
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
    /// A piece of the reply was larger than Dot reads, so the reply was cut and not kept.
    Oversized,
    /// Dot itself failed during the reply: a bug, reported rather than left waiting. The
    /// conversation starts afresh.
    Crashed,
}

/// Error from a [`Chat`]: what failed, the technical cause when there is one, and what the reply
/// had consumed before it failed.
///
/// `Display` writes the identifier of the failure's [`Text`]; the cause, from a library, Claude
/// Code or Anthropic, is in English.
#[derive(Debug)]
pub struct ReplyError {
    failure: ReplyFailure,
    cause: Option<Cause>,
    spent: Reply,
}

/// Conversation of one window with Claude, carried on from one prompt to the next.
pub trait Chat: Send {
    /// Sends `prompt`, its text and its images, to Claude as the next message of the
    /// conversation, then hands the reply to `on_event` piece by piece, until Claude has
    /// finished.
    ///
    /// Claude may use tools along the way, each one announced and ended through `on_event`.
    /// Whatever writes, runs a command or reaches outside the session's folder first asks
    /// `permissions`, on this thread, and waits for the answer.
    ///
    /// Blocks until the reply ends, on the network or on Claude Code: call it off the interface
    /// thread. Taking the conversation by `&mut` keeps it to one reply at a time. `credential`
    /// says how to reach Claude. Raising `cancellation` ends the reply at once, a running tool
    /// included; what Claude wrote until then stays in the conversation. A prompt that fails
    /// before Claude writes anything leaves the conversation as it was, so that the next prompt
    /// does not carry it.
    ///
    /// # Errors
    ///
    /// Returns a [`ReplyError`] if Claude cannot be reached, refuses the request or the
    /// credential, declines to answer, if a piece of the reply is too large to read, or if the
    /// reply is stopped through `cancellation`. The error tells what the reply consumed until
    /// then.
    fn reply(
        &mut self,
        prompt: Prompt,
        credential: &Credential,
        cancellation: &Cancellation,
        permissions: &dyn Permissions,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError>;

    /// Forgets the conversation: the next prompt starts a new one, with an empty context.
    fn reset(&mut self);
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
            Self::Oversized => Text::ReplyOversized,
            Self::Crashed => Text::ReplyCrashed,
        }
    }
}

impl ReplyError {
    /// Creates an error without technical cause, for a reply that consumed nothing.
    #[must_use]
    pub fn new(failure: ReplyFailure) -> Self {
        Self {
            failure,
            cause: None,
            spent: Reply::default(),
        }
    }

    /// Creates an error with the technical cause behind it, for a reply that consumed nothing.
    #[must_use]
    pub fn caused_by(failure: ReplyFailure, cause: impl Into<Cause>) -> Self {
        Self {
            failure,
            cause: Some(cause.into()),
            spent: Reply::default(),
        }
    }

    /// Returns the same error for a reply that consumed `spent` before it failed.
    #[must_use]
    pub const fn with_spent(mut self, spent: Reply) -> Self {
        self.spent = spent;
        self
    }

    /// Returns what failed.
    #[must_use]
    pub const fn failure(&self) -> ReplyFailure {
        self.failure
    }

    /// Returns what the reply consumed before it failed: its cost counts like a finished one.
    #[must_use]
    pub const fn spent(&self) -> Reply {
        self.spent
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
