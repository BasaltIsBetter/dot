use std::borrow::Cow;

use crate::settings::OutputCompaction;

/// What starts a command whose output Claude wants untouched, such as `DOT_RAW=1 cargo test`.
///
/// It sets a variable the command ignores; a [`Compactor`] leaves such an output as it is.
pub const RAW_OUTPUT_MARK: &str = "DOT_RAW=1";

/// Last line of an output a [`Compactor`] cut: it says how to read the whole of it.
///
/// Whoever kept the whole output somewhere may put the place there instead.
pub const CUT_OUTPUT_NOTE: &str =
    "[cut: start the command with DOT_RAW=1 to read its output whole]";

/// Name of the folder where outputs that were cut are kept whole.
///
/// A command that names this folder reads one of them: a [`Compactor`] leaves its output as it
/// is, like a command that starts with [`RAW_OUTPUT_MARK`].
pub const WHOLE_OUTPUTS_FOLDER: &str = "dot-whole";

/// Where an output that Claude is about to read comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin<'a> {
    /// A shell command that ran to its end, as Claude wrote it, whatever its exit status; its
    /// errors are mixed into its output.
    Command(&'a str),
    /// Lines that match a search, each written `path:line:text`.
    Matches,
    /// File paths, one per line.
    Paths,
}

/// Shortens what Claude reads from a tool, without changing what it tells Claude.
///
/// Every token of an output is read again with each later request of the conversation, so an
/// output is shortened once, before it enters the conversation, and never afterwards.
pub trait Compactor: Send + Sync {
    /// Returns a shorter text that tells Claude the same as `output`, or `None` when `output`
    /// stays as it is.
    ///
    /// The contents of a file and a diff are never rewritten. Whatever is left out is counted in
    /// square brackets, such as `[+12 lines: build progress]`. A long output may be cut, by the
    /// owner's choice: each cut is said where it was made, and such an output ends with
    /// [`CUT_OUTPUT_NOTE`]. A command that starts with [`RAW_OUTPUT_MARK`], or names
    /// [`WHOLE_OUTPUTS_FOLDER`], keeps its output whole.
    fn compact(&self, origin: Origin<'_>, output: &str) -> Option<String>;

    /// Returns `output`, a terminal's output as a program wrote it, the way a person reads it on
    /// screen: colours and window titles gone, and each line redrawn in place (`\r`) in its last
    /// state.
    ///
    /// For a person who watches a command run, never for Claude: nothing is counted or cut.
    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str>;

    /// Shortens later outputs as `compaction` says, from the next call of
    /// [`Compactor::compact`] on.
    fn tune(&mut self, compaction: OutputCompaction);
}

#[cfg(test)]
#[path = "../../../tests/dot-api/compaction.rs"]
mod tests;
