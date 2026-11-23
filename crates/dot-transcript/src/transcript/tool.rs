use std::time::Instant;

use crate::code::CodeBlock;

/// The pictogram a tool shows in the gutter, by what the tool does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    /// Reads a file or a folder.
    Read,
    /// Looks for files or for text in them.
    Search,
    /// Changes part of a file.
    Edit,
    /// Creates or rewrites a file.
    Create,
    /// Runs a command.
    Command,
    /// Hands work to other agents.
    Agents,
    /// Reaches the web.
    Web,
    /// Any other tool.
    Other,
}

impl Glyph {
    /// Returns the character that stands for the glyph in a line of text; the interface draws
    /// those its font lacks as pictograms.
    #[must_use]
    pub const fn mark(self) -> char {
        match self {
            Self::Read => '▤',
            Self::Search => '⌕',
            Self::Edit => '✎',
            Self::Create => '✚',
            Self::Command => '⌘',
            Self::Agents => '⧉',
            Self::Web => '◎',
            Self::Other => '◇',
        }
    }
}

/// A tool Claude called, as the gutter beside the reply shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    /// What the tool does.
    pub glyph: Glyph,
    /// What it works on, short: a file name, a pattern, a command.
    pub label: String,
}

/// A tool that still runs, with what it printed last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running<'a> {
    /// The identifier it was started with.
    pub id: &'a str,
    /// What it does and on what.
    pub tool: &'a Tool,
    /// When it started.
    pub started: Instant,
    /// The last lines it printed, oldest first; none before it prints.
    pub output: &'a [String],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) tool: Tool,
    pub(crate) result: Option<String>,
    pub(crate) started: Instant,
    pub(crate) output: Vec<String>,
    pub(crate) code: Option<CodeBlock>,
}
