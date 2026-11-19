/// What Claude asks one of Dot's own tools to do, read from its tool call and checked.
///
/// Paths are as Claude wrote them: absolute, or relative to the session's folder. Nothing here
/// is trusted: the [`Toolbox`](crate::Toolbox) that runs a call resolves its paths and asks the
/// user before anything leaves the folder, writes or runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolCall {
    /// Shows a file with its line numbers, an image, or the entries of a folder.
    View {
        /// File or folder to show.
        path: String,
        /// Lines of a text file to show; the whole file when `None`.
        lines: Option<LineRange>,
    },
    /// Creates a file, or replaces all of its text.
    Create {
        /// File to write.
        path: String,
        /// Whole text of the file.
        text: String,
    },
    /// Replaces the only occurrence of a text in a file.
    Replace {
        /// File to change.
        path: String,
        /// Text to find, exactly once in the file.
        old: String,
        /// Text to put in its place.
        new: String,
    },
    /// Inserts text after a line of a file.
    Insert {
        /// File to change.
        path: String,
        /// Line after which the text goes, counted from 1; 0 puts it at the start.
        after_line: u64,
        /// Text to insert.
        text: String,
    },
    /// Runs a shell command in the shell's current folder.
    Run {
        /// Command line, as Claude wrote it.
        command: String,
    },
    /// Starts the shell afresh, back in the session's folder.
    RestartShell,
    /// Lists the files whose path matches a glob pattern.
    FindFiles {
        /// Glob pattern, such as `src/**/*.rs`.
        pattern: String,
        /// Folder to search from; the session's folder when `None`.
        folder: Option<String>,
    },
    /// Searches the text of files for a regular expression.
    SearchText(TextSearch),
}

/// Lines of a file, counted from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineRange {
    /// First line shown.
    pub first: u64,
    /// Last line shown; up to the end of the file when `None`.
    pub last: Option<u64>,
}

/// A search through the text of files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSearch {
    /// Regular expression to look for.
    pub pattern: String,
    /// File or folder to search; the session's folder when `None`.
    pub folder: Option<String>,
    /// Glob pattern the searched files' paths must match, such as `*.rs`.
    pub files: Option<String>,
    /// Whether letter case is ignored.
    pub ignore_case: bool,
    /// What the search gives back.
    pub show: SearchShow,
}

/// What a [`TextSearch`] gives back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchShow {
    /// The paths of the files that match.
    #[default]
    Files,
    /// Each matching line, with its file and line number.
    Lines,
    /// How many lines match in each file.
    Counts,
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/tool/call.rs"]
mod tests;
