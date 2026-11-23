use std::collections::BTreeSet;

use dot_api::{Hunk, PatchLine};
use dot_transcript::NEW_LINE;

const LARGEST_COMPARISON: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Toward {
    Claude,
    Before,
    Yours,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Lines {
    pub(crate) changed: Vec<bool>,
    pub(crate) removed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Versions {
    before: Option<String>,
    claude: String,
    stash: Option<String>,
    created: bool,
    by_claude: BTreeSet<String>,
    claude_count: usize,
}

impl Lines {
    pub(crate) fn count(&self) -> usize {
        let changed = self.changed.iter().filter(|&&changed| changed).count();
        changed.max(self.removed)
    }
}

impl Versions {
    pub(crate) fn new(claude: String, changes: &[Vec<Hunk>], created: bool) -> Self {
        let before = (!created)
            .then(|| before_claude(&claude, changes))
            .flatten();
        let (by_claude, claude_count) = match &before {
            Some(before) => {
                let lines = changed_lines(before, &claude);
                let texts = claude
                    .split(NEW_LINE)
                    .zip(&lines.changed)
                    .filter(|&(_, &changed)| changed)
                    .map(|(line, _)| line.to_owned())
                    .collect();
                (texts, lines.count())
            }
            None if created => (BTreeSet::new(), claude.split(NEW_LINE).count()),
            None => {
                let added: Vec<&String> = changes
                    .iter()
                    .flatten()
                    .flat_map(|hunk| &hunk.lines)
                    .filter_map(|line| match line {
                        PatchLine::Added(text) => Some(text),
                        _ => None,
                    })
                    .collect();
                let count = added.len();
                (added.into_iter().cloned().collect(), count)
            }
        };
        Self {
            before,
            claude,
            stash: None,
            created,
            by_claude,
            claude_count,
        }
    }

    pub(crate) fn back(&self, current: &str) -> Option<Toward> {
        if self.before.as_deref() == Some(current) {
            return None;
        }
        if current != self.claude {
            return Some(Toward::Claude);
        }
        self.before
            .as_ref()
            .filter(|before| **before != self.claude)
            .map(|_| Toward::Before)
    }

    pub(crate) fn forward(&self, current: &str) -> Option<Toward> {
        let at_the_start = self.before.as_deref() == Some(current);
        if at_the_start && current != self.claude {
            return Some(Toward::Claude);
        }
        let yours = self.stash.as_ref().filter(|stash| **stash != self.claude);
        (current == self.claude && yours.is_some()).then_some(Toward::Yours)
    }

    pub(crate) fn text(&self, toward: Toward) -> &str {
        match toward {
            Toward::Claude => &self.claude,
            Toward::Before => self.before.as_deref().unwrap_or(&self.claude),
            Toward::Yours => self.stash.as_deref().unwrap_or(&self.claude),
        }
    }

    pub(crate) fn leave(&mut self, current: &str) {
        if current != self.claude {
            self.stash = Some(current.to_owned());
        }
    }

    pub(crate) fn mine(&self, current: &str) -> Lines {
        changed_lines(&self.claude, current)
    }

    pub(crate) fn by_claude(&self, line: &str) -> bool {
        self.by_claude.contains(line)
    }

    pub(crate) const fn claude_count(&self) -> usize {
        self.claude_count
    }

    pub(crate) const fn created(&self) -> bool {
        self.created
    }
}

fn before_claude(claude: &str, changes: &[Vec<Hunk>]) -> Option<String> {
    let mut lines: Vec<String> = claude.split(NEW_LINE).map(str::to_owned).collect();
    for hunks in changes.iter().rev() {
        for hunk in hunks.iter().rev() {
            undo(&mut lines, hunk)?;
        }
    }
    Some(lines.join(&NEW_LINE.to_string()))
}

fn undo(lines: &mut Vec<String>, hunk: &Hunk) -> Option<()> {
    let after: Vec<&str> = hunk
        .lines
        .iter()
        .filter_map(|line| match line {
            PatchLine::Same(text) | PatchLine::Added(text) => Some(text.as_str()),
            PatchLine::Removed(_) => None,
        })
        .collect();
    let before = hunk.lines.iter().filter_map(|line| match line {
        PatchLine::Same(text) | PatchLine::Removed(text) => Some(text.clone()),
        PatchLine::Added(_) => None,
    });
    let start = usize::try_from(hunk.new_start).ok()?;
    let start = if after.is_empty() {
        start
    } else {
        start.checked_sub(1)?
    };
    let end = start.checked_add(after.len())?;
    let found = lines.get(start..end)?;
    if !found.iter().map(String::as_str).eq(after.iter().copied()) {
        return None;
    }
    lines.splice(start..end, before);
    Some(())
}

pub(crate) fn changed_lines(before: &str, after: &str) -> Lines {
    let old: Vec<&str> = before.split(NEW_LINE).collect();
    let new: Vec<&str> = after.split(NEW_LINE).collect();
    let head = old
        .iter()
        .zip(&new)
        .take_while(|(old, new)| old == new)
        .count();
    let tail = old[head..]
        .iter()
        .rev()
        .zip(new[head..].iter().rev())
        .take_while(|(old, new)| old == new)
        .count();
    let old_middle = &old[head..old.len() - tail];
    let new_middle = &new[head..new.len() - tail];
    let mut changed = vec![false; new.len()];
    let kept = if old_middle.len().saturating_mul(new_middle.len()) <= LARGEST_COMPARISON {
        let kept = common(old_middle, new_middle);
        for (flag, kept) in changed[head..].iter_mut().zip(&kept) {
            *flag = !kept;
        }
        kept.iter().filter(|&&kept| kept).count()
    } else {
        changed[head..new.len() - tail].fill(true);
        0
    };
    Lines {
        changed,
        removed: old_middle.len() - kept,
    }
}

fn common(old: &[&str], new: &[&str]) -> Vec<bool> {
    let width = new.len() + 1;
    let mut longest = vec![0_u32; (old.len() + 1) * width];
    for row in (0..old.len()).rev() {
        for column in (0..new.len()).rev() {
            longest[row * width + column] = if old[row] == new[column] {
                longest[(row + 1) * width + column + 1] + 1
            } else {
                longest[(row + 1) * width + column].max(longest[row * width + column + 1])
            };
        }
    }
    let mut kept = vec![false; new.len()];
    let (mut row, mut column) = (0, 0);
    while row < old.len() && column < new.len() {
        if old[row] == new[column] {
            kept[column] = true;
            row += 1;
            column += 1;
        } else if longest[(row + 1) * width + column] >= longest[row * width + column + 1] {
            row += 1;
        } else {
            column += 1;
        }
    }
    kept
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/editor/versions.rs"]
mod tests;
