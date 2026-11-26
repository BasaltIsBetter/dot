use std::borrow::Cow;

use dot_api::{Compactor, Origin, OutputCompaction};

use crate::command::CommandLine;
use crate::cut::{Burst, Limits};
use crate::report::LineStream;
use crate::{cut, noise, report, shape};

const SHORTEST_WORTH_READING: usize = 400;
const SMALLEST_GAIN: usize = 80;

type Stage<'a> = &'a dyn Fn(&str) -> Option<String>;

/// [`Compactor`] that recognises what an output looks like and writes it shorter, as its
/// [`OutputCompaction`] says.
///
/// It leaves alone anything under 400 bytes and the commands that start with
/// [`dot_api::RAW_OUTPUT_MARK`] or name [`dot_api::WHOLE_OUTPUTS_FOLDER`], and keeps a shorter
/// text only when that saves at least 80 bytes. It never rewrites the contents of files (what
/// `cat`, `sed`, `head` and the like print) nor diffs.
///
/// Each part of its work is on or off. Cleaning removes the colours of the terminal and the
/// states a line had before it was redrawn in place. Rewriting writes repeated paths once,
/// folds lists of files, removes the spaces of indented JSON and shortens trees, tables,
/// `git status` and `git log`, without losing anything. Counting turns the progress lines of the
/// tools a command names, and the tests that passed, into counts.
///
/// By the owner's choice it also cuts an output above the bounds of its
/// [`CutLevel`](dot_api::CutLevel): 7,000 bytes (10,000 for the contents of files and diffs) at
/// the measured level, when that removes
/// at least 4,000 bytes, and says each cut where it is made. First goes what the shape of the
/// output can spare: the middle of a stack trace, the commits of a log beyond one line each,
/// the names of one folder and the matches of one file past the first, lines alike but for
/// their numbers, the unchanged lines of a diff away from a change. Then the end of a line above
/// 400 bytes (1,000 in the contents of files, prose excepted), then the middle of the output;
/// for the contents of a file, the cut lists its lines that open a block, or the headings of a
/// document. An output that was cut ends with [`dot_api::CUT_OUTPUT_NOTE`].
///
/// [`ShapeCompactor::default`] does all of it, at the measured bounds
/// ([`OutputCompaction::FULL`]).
///
/// # Examples
///
/// ```
/// use dot_api::{Compactor, Origin, OutputCompaction, Toggle};
/// use dot_compact::ShapeCompactor;
///
/// let found: String = (1..=20)
///     .map(|line| format!("crates/dot-api/src/session.rs:{line}:    let spent = usage.cost();\n"))
///     .collect();
///
/// let compactor = ShapeCompactor::default();
/// let shorter = compactor.compact(Origin::Command("grep -rn spent crates"), &found);
///
/// assert!(shorter.is_some_and(|shorter| shorter.starts_with("crates/dot-api/src/session.rs\n1:")));
/// assert_eq!(compactor.compact(Origin::Command("cat notes.txt"), &found), None);
///
/// let untouched = ShapeCompactor::new(OutputCompaction {
///     rewriting: Toggle::Off,
///     ..OutputCompaction::FULL
/// });
/// assert_eq!(untouched.compact(Origin::Command("grep -rn spent crates"), &found), None);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ShapeCompactor {
    compaction: OutputCompaction,
}

impl Default for ShapeCompactor {
    fn default() -> Self {
        Self::new(OutputCompaction::FULL)
    }
}

impl ShapeCompactor {
    /// Creates the compactor that shortens outputs as `compaction` says.
    #[must_use]
    pub const fn new(compaction: OutputCompaction) -> Self {
        Self { compaction }
    }

    /// Returns whether what `command` prints is never touched: Claude asked for it whole with
    /// [`dot_api::RAW_OUTPUT_MARK`], or the command reads an output kept whole in
    /// [`dot_api::WHOLE_OUTPUTS_FOLDER`].
    #[must_use]
    pub fn keeps_whole(command: &str) -> bool {
        CommandLine::read(command).keeps_raw()
    }

    /// Returns what shortens the output of `command` while it still runs: the colours of the
    /// terminal and the lines redrawn in place go when cleaning is on, and the progress lines
    /// of the tools the command names become counts when counting is. `None` when both are
    /// off, and for a command that prints the contents of a file or a diff, or keeps its output
    /// whole: what it prints passes as it comes.
    #[must_use]
    pub fn stream(&self, command: &str) -> Option<LineStream> {
        let compaction = &self.compaction;
        if !compaction.cleaning.is_on() && !compaction.progress.is_on() {
            return None;
        }
        let command = CommandLine::read(command);
        let passes_as_it_comes = command.keeps_raw() || command.prints_content();
        (!passes_as_it_comes)
            .then(|| LineStream::of(&command, compaction.cleaning, compaction.progress))
    }

    /// Returns what cuts the output of `command` while it still runs, within the same bounds
    /// as a finished output: those of the contents of a file when the command prints one.
    /// `None` when such an output is never cut.
    #[must_use]
    pub fn burst(&self, command: &str) -> Option<Burst> {
        let limits = if CommandLine::read(command).prints_content() {
            self.contents_limits()
        } else {
            self.outputs_limits()
        };
        limits.map(Burst::within)
    }

    fn contents_limits(self) -> Option<Limits> {
        self.compaction
            .contents_cut
            .of_contents()
            .map(Limits::of_contents)
    }

    fn outputs_limits(self) -> Option<Limits> {
        self.compaction
            .outputs_cut
            .of_outputs()
            .map(Limits::of_outputs)
    }

    fn document_limits(self) -> Option<Limits> {
        self.compaction
            .outputs_cut
            .of_outputs()
            .map(Limits::of_a_document)
    }

    fn of_command(self, line: &str, output: &str) -> Option<String> {
        let command = CommandLine::read(line);
        if command.keeps_raw() {
            return None;
        }
        if command.prints_content() {
            return cut_within(output, self.contents_limits());
        }

        let rewriting = self.compaction.rewriting.is_on();
        let counting = self.compaction.progress.is_on();
        let mut text = if self.compaction.cleaning.is_on() {
            noise::clean(output)
        } else {
            Cow::Borrowed(output)
        };
        if shape::is_diff(&text) {
            return cut_within(&text, self.contents_limits()).or_else(|| owned(text));
        }
        if rewriting && let Some(minified) = shape::minify(&text) {
            return cut_within(&minified, self.document_limits()).or(Some(minified));
        }
        if shape::is_document(&text) {
            return cut_within(&text, self.document_limits()).or_else(|| owned(text));
        }
        let stages: [(bool, Stage<'_>); 10] = [
            (rewriting, &|text| report::rewrite(&command, text)),
            (counting, &|text| report::count(&command, text)),
            (rewriting, &shape::minify_after_lines),
            (rewriting, &noise::shorten_rulers),
            (rewriting, &shape::flatten_tree),
            (rewriting, &shape::group_matches),
            (rewriting, &shape::name_folder),
            (rewriting, &shape::group_paths),
            (rewriting, &shape::name_neighbours),
            (rewriting, &noise::collapse),
        ];
        for (_, stage) in stages.into_iter().filter(|(on, _)| *on) {
            if let Some(shorter) = stage(&text) {
                text = Cow::Owned(shorter);
            }
        }
        cut_within(&text, self.outputs_limits()).or_else(|| owned(text))
    }
}

impl Compactor for ShapeCompactor {
    fn compact(&self, origin: Origin<'_>, output: &str) -> Option<String> {
        if output.len() < SHORTEST_WORTH_READING {
            return None;
        }
        let rewriting = self.compaction.rewriting.is_on();
        let shorter = match origin {
            Origin::Command(line) => self.of_command(line, output),
            Origin::Matches if rewriting => shape::group_matches(output),
            Origin::Paths if rewriting => shape::group_paths(output),
            Origin::Matches | Origin::Paths => None,
        }?;
        (shorter.len() + SMALLEST_GAIN <= output.len()).then_some(shorter)
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        noise::clean(output)
    }

    fn tune(&mut self, compaction: OutputCompaction) {
        self.compaction = compaction;
    }
}

fn cut_within(text: &str, limits: Option<Limits>) -> Option<String> {
    cut::cut(text, &limits?)
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
