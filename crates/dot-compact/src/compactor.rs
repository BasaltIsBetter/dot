use std::borrow::Cow;

use dot_api::{Compactor, Origin};

use crate::command::CommandLine;
use crate::cut::Burst;
use crate::report::LineStream;
use crate::{cut, noise, report, shape};

const SHORTEST_WORTH_READING: usize = 400;
const SMALLEST_GAIN: usize = 80;

type Stage<'a> = &'a dyn Fn(&str) -> Option<String>;

/// [`Compactor`] that recognises what an output looks like and writes it shorter.
///
/// It leaves alone anything under 400 bytes and the commands that start with
/// [`dot_api::RAW_OUTPUT_MARK`] or name [`dot_api::WHOLE_OUTPUTS_FOLDER`], and keeps a shorter
/// text only when that saves at least 80 bytes. It never rewrites the contents of files (what
/// `cat`, `sed`, `head` and the like print) nor diffs.
///
/// By the owner's choice it also cuts an output above 7,000 bytes (10,000 for the contents of
/// files and diffs), when that removes at least 4,000 bytes, and says each cut where it is
/// made. First goes what the shape of the output can spare: the middle of a stack trace, the
/// commits of a log beyond one line each, the names of one folder and the matches of one file
/// past the first, lines alike but for their numbers, the unchanged lines of a diff away from
/// a change. Then the end of a line above 400 bytes (1,000 in the contents of files, prose
/// excepted), then the middle of the output; for the contents of a file, the cut lists its
/// lines that open a block, or the headings of a document. An output that was cut ends with
/// [`dot_api::CUT_OUTPUT_NOTE`].
///
/// # Examples
///
/// ```
/// use dot_api::{Compactor, Origin};
/// use dot_compact::ShapeCompactor;
///
/// let found: String = (1..=20)
///     .map(|line| format!("crates/dot-api/src/session.rs:{line}:    let spent = usage.cost();\n"))
///     .collect();
///
/// let shorter = ShapeCompactor.compact(Origin::Command("grep -rn spent crates"), &found);
///
/// assert!(shorter.is_some_and(|shorter| shorter.starts_with("crates/dot-api/src/session.rs\n1:")));
/// assert_eq!(ShapeCompactor.compact(Origin::Command("cat notes.txt"), &found), None);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ShapeCompactor;

impl ShapeCompactor {
    /// Returns whether what `command` prints is never touched: Claude asked for it whole with
    /// [`dot_api::RAW_OUTPUT_MARK`], or the command reads an output kept whole in
    /// [`dot_api::WHOLE_OUTPUTS_FOLDER`].
    #[must_use]
    pub fn keeps_whole(command: &str) -> bool {
        CommandLine::read(command).keeps_raw()
    }

    /// Returns what shortens the output of `command` while it still runs: the colours of the
    /// terminal and the lines redrawn in place go, and the progress lines of the tools the
    /// command names become counts. `None` for a command that prints the contents of a file or
    /// a diff, or keeps its output whole: what it prints passes as it comes.
    #[must_use]
    pub fn stream(command: &str) -> Option<LineStream> {
        let command = CommandLine::read(command);
        let passes_as_it_comes = command.keeps_raw() || command.prints_content();
        (!passes_as_it_comes).then(|| LineStream::of(&command))
    }

    /// Returns what cuts the output of `command` while it still runs, within the same bounds
    /// as a finished output: those of the contents of a file when the command prints one.
    #[must_use]
    pub fn burst(command: &str) -> Burst {
        if CommandLine::read(command).prints_content() {
            return Burst::within(&cut::CONTENT);
        }
        Burst::within(&cut::OTHER)
    }
}

impl Compactor for ShapeCompactor {
    fn compact(&self, origin: Origin<'_>, output: &str) -> Option<String> {
        if output.len() < SHORTEST_WORTH_READING {
            return None;
        }
        let shorter = match origin {
            Origin::Command(line) => of_command(line, output),
            Origin::Matches => shape::group_matches(output),
            Origin::Paths => shape::group_paths(output),
        }?;
        (shorter.len() + SMALLEST_GAIN <= output.len()).then_some(shorter)
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        noise::clean(output)
    }
}

fn of_command(line: &str, output: &str) -> Option<String> {
    let command = CommandLine::read(line);
    if command.keeps_raw() {
        return None;
    }
    if command.prints_content() {
        return cut::cut(output, &cut::CONTENT);
    }

    let mut text = noise::clean(output);
    if shape::is_diff(&text) {
        return cut::cut(&text, &cut::CONTENT).or_else(|| owned(text));
    }
    if let Some(minified) = shape::minify(&text) {
        return cut::cut(&minified, &cut::ONE_DOCUMENT).or(Some(minified));
    }
    if shape::is_document(&text) {
        return cut::cut(&text, &cut::ONE_DOCUMENT).or_else(|| owned(text));
    }
    let stages: [Stage<'_>; 9] = [
        &|text| report::compact(&command, text),
        &shape::minify_after_lines,
        &noise::shorten_rulers,
        &shape::flatten_tree,
        &shape::group_matches,
        &shape::name_folder,
        &shape::group_paths,
        &shape::name_neighbours,
        &noise::collapse,
    ];
    for stage in stages {
        if let Some(shorter) = stage(&text) {
            text = Cow::Owned(shorter);
        }
    }
    cut::cut(&text, &cut::OTHER).or_else(|| owned(text))
}

fn owned(text: Cow<'_, str>) -> Option<String> {
    match text {
        Cow::Owned(text) => Some(text),
        Cow::Borrowed(_) => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-compact/compactor.rs"]
mod tests;
