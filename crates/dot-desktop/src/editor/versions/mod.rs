mod changed;

use std::collections::BTreeSet;

use dot_api::{Hunk, PatchLine};
use dot_transcript::NEW_LINE;

pub(crate) use self::changed::{Lines, changed_lines};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Toward {
    Claude,
    Before,
    Yours,
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

impl Versions {
    pub(crate) fn new(claude: String, changes: Option<&[Vec<Hunk>]>, created: bool) -> Self {
        let before = changes
            .filter(|_| !created)
            .and_then(|changes| before_claude(&claude, changes));
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
                    .unwrap_or_default()
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

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/versions/mod.rs"]
mod tests;
