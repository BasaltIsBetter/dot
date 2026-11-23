use std::error::Error;
use std::fmt;

use crate::text::Text;

/// Largest file a person edits by hand in Dot, in bytes: 1 MiB.
pub const LARGEST_EDITED: u64 = 1_024 * 1_024;

/// The files a person reads and changes by hand in Dot: those of the session's folder, and those
/// Claude changed elsewhere.
///
/// A relative path is taken from the session's folder and refused when it leads out of it,
/// through `..` or a link; an absolute path names a file wherever it is. The interface only asks
/// for files of the folder and files Claude changed. Implementations touch the disk: call them
/// off the interface's thread.
pub trait Workspace: Send + Sync {
    /// Returns the text of the file at `path`, its Windows line ends turned into line breaks.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError`] when a relative path leads out of the folder, when the path
    /// names no regular file, when the file is larger than [`LARGEST_EDITED`] or is not UTF-8
    /// text, or when the system refuses to read it.
    fn read(&self, path: &str) -> Result<String, WorkspaceError>;

    /// Replaces the file at `path` with `text` in one step, keeping its permissions, and its
    /// Windows line ends when it had them.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError`] when a relative path leads out of the folder, when the path
    /// names no regular file, or when the system refuses to write it; the file is then left as
    /// it was.
    fn write(&self, path: &str, text: &str) -> Result<(), WorkspaceError>;

    /// Returns the entries of the folder at `path`, `""` for the session's folder itself:
    /// sorted by name whatever their case, folders ending with `/`, hidden ones left out.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError`] when a relative path leads out of the folder, or when the path
    /// names no folder the system lets Dot read.
    fn list(&self, path: &str) -> Result<Vec<String>, WorkspaceError>;
}

/// Why a file of the [`Workspace`] could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceError {
    /// A relative path leads out of the session's folder, or the session has no folder.
    Outside,
    /// The path names a folder, a pipe, a socket or a device, not a file.
    NotAFile,
    /// The file is larger than [`LARGEST_EDITED`].
    TooLarge,
    /// The file is not UTF-8 text.
    NotText,
    /// The system refused, with its own words.
    Failed(String),
}

impl WorkspaceError {
    /// Returns what the interface tells the user.
    #[must_use]
    pub const fn text(&self) -> Text {
        match self {
            Self::Outside => Text::EditorOutside,
            Self::NotAFile => Text::EditorNotAFile,
            Self::TooLarge => Text::EditorTooLarge,
            Self::NotText => Text::EditorNotText,
            Self::Failed(_) => Text::EditorFailed,
        }
    }
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(cause) => write!(formatter, "{}: {cause}", self.text().id()),
            other => formatter.write_str(other.text().id()),
        }
    }
}

impl Error for WorkspaceError {}

#[cfg(test)]
#[path = "../../../../tests/dot-api/file/workspace.rs"]
mod tests;
