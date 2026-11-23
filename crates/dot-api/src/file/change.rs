const CONTEXT_LINES: usize = 8;
const LINE_BREAK: char = '\n';
const CARRIAGE_RETURN: char = '\r';

/// A change a tool made to a file, line by line, as the conversation shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileChange {
    /// The file, as shown: relative to the session's folder when it lies inside it.
    pub path: String,
    /// Whether the change created the file.
    pub created: bool,
    /// The parts that changed, first to last, each with a few unchanged lines around it.
    pub hunks: Vec<Hunk>,
    /// Where a created file sits among the files of its folder, when Dot could tell.
    pub siblings: Option<Siblings>,
}

/// A part of a file that changed, with the unchanged lines around it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hunk {
    /// Number of the hunk's first line in the file before the change, from 1; 0 when the file
    /// had no line there.
    pub old_start: u64,
    /// Number of the hunk's first line in the file after the change, from 1.
    pub new_start: u64,
    /// The lines, in the order of the file.
    pub lines: Vec<PatchLine>,
}

/// A line of a [`Hunk`], without its line break.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchLine {
    /// A line the change left as it was.
    Same(String),
    /// A line the change took away.
    Removed(String),
    /// A line the change added.
    Added(String),
}

/// The neighbours of a created file in its folder, in the order a listing shows them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Siblings {
    /// The folder, as shown, ending with `/`.
    pub folder: String,
    /// The two entries listed just before the file, at most.
    pub before: Vec<String>,
    /// The two entries listed just after the file, at most.
    pub after: Vec<String>,
}

impl FileChange {
    /// Returns the change from the text `before` to the text `after` of the file at `path`: one
    /// hunk, from the first line that differs to the last, with eight unchanged lines around
    /// it at most; no hunk when nothing changed.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_api::{FileChange, PatchLine};
    ///
    /// let change = FileChange::between("a.rs", "a\nb\nc\n", "a\nB\nc\n", false);
    ///
    /// assert_eq!(change.hunks[0].old_start, 1);
    /// assert_eq!(change.hunks[0].lines[1], PatchLine::Removed("b".to_owned()));
    /// assert_eq!(change.hunks[0].lines[2], PatchLine::Added("B".to_owned()));
    /// ```
    #[must_use]
    pub fn between(path: &str, before: &str, after: &str, created: bool) -> Self {
        let old: Vec<&str> = lines_of(before);
        let new: Vec<&str> = lines_of(after);
        let same_head = old
            .iter()
            .zip(&new)
            .take_while(|(old, new)| old == new)
            .count();
        let same_tail = old[same_head..]
            .iter()
            .rev()
            .zip(new[same_head..].iter().rev())
            .take_while(|(old, new)| old == new)
            .count();
        let hunks = if same_head == old.len() && same_head == new.len() {
            Vec::new()
        } else {
            vec![hunk(&old, &new, same_head, same_tail)]
        };
        Self {
            path: path.to_owned(),
            created,
            hunks,
            siblings: None,
        }
    }

    /// Returns the change with up to eight unchanged lines around each hunk, read from
    /// `original`, the file before the change; hunks that come to touch are joined into one.
    ///
    /// A hunk keeps what it had: lines are only added around it, and never past either end of
    /// `original`. A hunk that names lines `original` does not have is left as it is.
    #[must_use]
    pub fn widened(mut self, original: &str) -> Self {
        let lines = lines_of(original);
        let mut joined: Vec<Hunk> = Vec::with_capacity(self.hunks.len());
        for hunk in self.hunks {
            let hunk = widen(hunk, &lines);
            match joined.last_mut() {
                Some(last) if old_range(last).end >= old_range(&hunk).start => join(last, hunk),
                _ => joined.push(hunk),
            }
        }
        self.hunks = joined;
        self
    }
}

fn widen(mut hunk: Hunk, lines: &[&str]) -> Hunk {
    let range = old_range(&hunk);
    if range.end > lines.len() {
        return hunk;
    }
    let (first, end) = (range.start, range.end);
    let leading = hunk.lines.iter().take_while(|line| is_same(line)).count();
    let trailing = hunk
        .lines
        .iter()
        .rev()
        .take_while(|line| is_same(line))
        .count();
    let before = CONTEXT_LINES.saturating_sub(leading).min(first);
    let after = CONTEXT_LINES
        .saturating_sub(trailing)
        .min(lines.len() - end);
    let same = |line: &&str| PatchLine::Same((*line).to_owned());
    let mut widened: Vec<PatchLine> = lines[first - before..first].iter().map(same).collect();
    widened.append(&mut hunk.lines);
    widened.extend(lines[end..end + after].iter().map(same));
    let before = u64::try_from(before).unwrap_or(u64::MAX);
    Hunk {
        old_start: u64::try_from(first).unwrap_or(u64::MAX) - before + 1,
        new_start: hunk.new_start.saturating_sub(before).max(1),
        lines: widened,
    }
}

fn join(last: &mut Hunk, next: Hunk) {
    let overlap = old_range(last).end.saturating_sub(old_range(&next).start);
    let leading = next.lines.iter().take_while(|line| is_same(line)).count();
    last.lines
        .extend(next.lines.into_iter().skip(overlap.min(leading)));
}

fn old_range(hunk: &Hunk) -> std::ops::Range<usize> {
    let count = hunk
        .lines
        .iter()
        .filter(|line| !matches!(line, PatchLine::Added(_)))
        .count();
    let start = usize::try_from(hunk.old_start).unwrap_or(usize::MAX);
    let first = if count == 0 {
        start
    } else {
        start.saturating_sub(1)
    };
    first..first + count
}

const fn is_same(line: &PatchLine) -> bool {
    matches!(line, PatchLine::Same(_))
}

fn hunk(old: &[&str], new: &[&str], same_head: usize, same_tail: usize) -> Hunk {
    let first = same_head.saturating_sub(CONTEXT_LINES);
    let old_changed_end = old.len() - same_tail;
    let new_changed_end = new.len() - same_tail;
    let old_end = (old_changed_end + CONTEXT_LINES).min(old.len());
    let owned = |line: &&str| (*line).to_owned();
    let mut lines: Vec<PatchLine> = old[first..same_head]
        .iter()
        .map(|line| PatchLine::Same(owned(line)))
        .collect();
    lines.extend(
        old[same_head..old_changed_end]
            .iter()
            .map(|line| PatchLine::Removed(owned(line))),
    );
    lines.extend(
        new[same_head..new_changed_end]
            .iter()
            .map(|line| PatchLine::Added(owned(line))),
    );
    lines.extend(
        old[old_changed_end..old_end]
            .iter()
            .map(|line| PatchLine::Same(owned(line))),
    );
    let number = |index: usize, of: &[&str]| {
        if of.is_empty() {
            0
        } else {
            u64::try_from(index + 1).unwrap_or(u64::MAX)
        }
    };
    Hunk {
        old_start: number(first, old),
        new_start: u64::try_from(first + 1).unwrap_or(u64::MAX),
        lines,
    }
}

fn lines_of(text: &str) -> Vec<&str> {
    text.split_terminator(LINE_BREAK)
        .map(|line| line.strip_suffix(CARRIAGE_RETURN).unwrap_or(line))
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/file/change.rs"]
mod tests;
