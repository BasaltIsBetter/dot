use serde::{Deserialize, Serialize};

use crate::chat::Image;
use crate::file::FileChange;
use crate::text::Text;

/// A tool Claude uses, as the conversation shows it: what it does and on what.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolUse {
    /// What the tool does.
    pub kind: ToolKind,
    /// What it works on: a path, a command, a pattern, an address, a query, or a task.
    pub subject: String,
}

/// What a tool does, among those Claude uses in Dot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    /// Reads a file, an image or a folder.
    Read,
    /// Creates or rewrites a file.
    Write,
    /// Changes part of a file.
    Edit,
    /// Runs a shell command.
    Run,
    /// Looks for files by name.
    FindFiles,
    /// Looks for text in files.
    SearchText,
    /// Fetches a web page.
    Fetch,
    /// Searches the web.
    SearchWeb,
    /// Hands a task to another agent.
    Delegate,
    /// Any other tool; the subject starts with its name.
    Other,
}

/// How a tool call ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolEnd {
    /// The tool ran, with what it measured.
    Done(Detail),
    /// The tool could not do what Claude asked; Claude reads why.
    Failed,
    /// The user refused to let it run.
    Refused,
    /// The reply was stopped before the tool finished.
    Stopped,
}

/// What a tool that ran measured, shown after it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Detail {
    /// Nothing worth a number.
    #[default]
    Nothing,
    /// Lines read or shown.
    Lines(u64),
    /// Files or folder entries found.
    Files(u64),
    /// Lines that match a search.
    Matches(u64),
    /// Lines a change added to and removed from a file.
    Changes {
        /// Lines added.
        added: u64,
        /// Lines removed.
        removed: u64,
    },
    /// Exit status of a command.
    Exit(i32),
}

/// What one of Dot's tools hands back to Claude.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutput {
    /// What Claude reads.
    pub content: ToolContent,
    /// Whether the tool failed or was refused, so that Claude reads the content as an error.
    pub is_error: bool,
    /// How the call ended, as the conversation shows it.
    pub end: ToolEnd,
    /// What the tool changed in a file, when it changed one; boxed, for an output that changed
    /// nothing to stay small.
    pub change: Option<Box<FileChange>>,
}

/// What Claude reads from a tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolContent {
    /// Text: a file with line numbers, a listing, an output, or an error.
    Text(String),
    /// An image file, read as it is.
    Image(Image),
}

impl ToolKind {
    /// Returns the text that names what the tool does, shown in front of its subject.
    #[must_use]
    pub const fn text(self) -> Text {
        match self {
            Self::Read => Text::ToolRead,
            Self::Write => Text::ToolWrite,
            Self::Edit => Text::ToolEdit,
            Self::Run => Text::ToolRun,
            Self::FindFiles => Text::ToolFindFiles,
            Self::SearchText => Text::ToolSearchText,
            Self::Fetch => Text::ToolFetch,
            Self::SearchWeb => Text::ToolSearchWeb,
            Self::Delegate => Text::ToolDelegate,
            Self::Other => Text::ToolOther,
        }
    }
}

impl ToolOutput {
    /// Creates the output of a tool that ran, measuring `detail`.
    #[must_use]
    pub fn done(text: impl Into<String>, detail: Detail) -> Self {
        Self {
            content: ToolContent::Text(text.into()),
            is_error: false,
            end: ToolEnd::Done(detail),
            change: None,
        }
    }

    /// Returns the output with the `change` the tool made to a file.
    #[must_use]
    pub fn changing(mut self, change: FileChange) -> Self {
        self.change = Some(Box::new(change));
        self
    }

    /// Creates the output of a tool that could not do what Claude asked, telling Claude why.
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self::error(reason, ToolEnd::Failed)
    }

    /// Creates the output of a tool the user refused to let run.
    #[must_use]
    pub fn refused() -> Self {
        Self::error(USER_REFUSED_THE_TOOL, ToolEnd::Refused)
    }

    /// Creates the output of a tool the user stopped along with the reply.
    #[must_use]
    pub fn stopped() -> Self {
        Self::error(USER_STOPPED_THE_REPLY, ToolEnd::Stopped)
    }

    fn error(reason: impl Into<String>, end: ToolEnd) -> Self {
        Self {
            content: ToolContent::Text(reason.into()),
            is_error: true,
            end,
            change: None,
        }
    }
}

/// Text Claude reads when the user refuses a tool call.
pub const USER_REFUSED_THE_TOOL: &str = "The user refused to let this tool run. Do not try to get around \
the refusal; ask the user what to do instead if you need it.";

/// Text Claude reads when the user stops the reply before a tool call ran or finished.
pub const USER_STOPPED_THE_REPLY: &str = "The user stopped the reply before this tool finished.";

#[cfg(test)]
#[path = "../../../../tests/dot-api/tool/report.rs"]
mod tests;
