use crate::file::FileChange;
use crate::tool::{ToolEnd, ToolUse};

/// Most lines of a running tool's output that [`ReplyEvent::ToolOutput`] carries: the last ones.
pub const LIVE_OUTPUT_LINES: usize = 3;

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
    /// A tool that still runs printed more: here are its latest lines, for a person to watch.
    ///
    /// Each one replaces the one before for the same call. Colours, window titles and the
    /// earlier states of a line redrawn in place are gone; nothing here reaches Claude.
    ToolOutput {
        /// Identifier of the call, as when it started.
        id: String,
        /// The last lines printed so far, oldest first, [`LIVE_OUTPUT_LINES`] at most; the
        /// last one may still be growing.
        lines: Vec<String>,
    },
    /// A tool changed a file, just before its call ends: the lines it changed, for a person to
    /// read.
    ToolChanged {
        /// Identifier of the call, as when it started.
        id: String,
        /// What changed in the file, boxed so that every other event stays small.
        change: Box<FileChange>,
    },
    /// A tool call ended.
    ToolEnded {
        /// Identifier of the call, as when it started.
        id: String,
        /// How it ended.
        end: ToolEnd,
    },
    /// An agent Claude handed a task to did something.
    Agent {
        /// Identifier of the call that handed the task over, a [`ToolKind::Delegate`] tool.
        ///
        /// [`ToolKind::Delegate`]: crate::ToolKind::Delegate
        id: String,
        /// What the agent did.
        event: AgentEvent,
    },
    /// The conversation grew long and was summarized: from here on Claude reads the summary and
    /// the latest messages instead of everything said before.
    Compacted {
        /// Tokens in Claude's context window before the summary.
        before: u64,
        /// Tokens the summary and what it keeps take.
        after: u64,
    },
}

/// What an agent that Claude handed a task to does, between the start and the end of the call
/// that handed it over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentEvent {
    /// The agent starts working.
    Started {
        /// What kind of agent it is, as Claude asked for it, such as `Explore`.
        kind: String,
    },
    /// The agent wrote a message.
    Wrote {
        /// Tokens the message took to write: its text, its thinking and its tools' input.
        tokens: u64,
    },
    /// What the agent says along the way, before its report.
    Said(String),
    /// The agent calls a tool.
    ToolStarted {
        /// Identifier of the agent's call.
        id: String,
        /// What the tool does, as shown.
        tool: ToolUse,
    },
    /// A tool of the agent ended.
    ToolEnded {
        /// Identifier of the agent's call, as when it started.
        id: String,
        /// How it ended.
        end: ToolEnd,
    },
    /// The agent hands its work back to Claude.
    Reported {
        /// The report Claude reads.
        text: String,
        /// Tokens the agent wrote over its whole task.
        tokens: u64,
    },
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/chat/event.rs"]
mod tests;
