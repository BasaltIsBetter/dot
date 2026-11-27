use std::time::Instant;

use crate::code::CodeBlock;

/// The pictogram of a tool, by what the tool does.
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

/// A tool Claude called: what it does and on what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    /// What the tool does.
    pub glyph: Glyph,
    /// What it works on, short: a file name, a pattern, a command.
    pub label: String,
}

/// The line that sums up a tool in the reply once it ended.
///
/// It holds on one row while it can: `cut` is shortened with `…` at its end so that `kept`
/// still shows whole, and shortened again each time the width changes. Wider than the row
/// on its own, `kept` wraps like any text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aside<'a> {
    /// What may be shortened, first on the line: a command and its glyph. Empty when nothing
    /// may be.
    pub cut: &'a str,
    /// What always shows whole after it: what the tool measured, how long it took.
    pub kept: &'a str,
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
    pub(crate) started: Instant,
    pub(crate) output: Vec<String>,
    pub(crate) code: Option<Box<CodeBlock>>,
    pub(crate) summary: Option<Box<Summary>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) cut: String,
    pub(crate) kept: String,
    pub(crate) replaced: Option<Replaced>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Replaced {
    pub(crate) before: String,
    pub(crate) at: Instant,
}
