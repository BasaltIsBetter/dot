use std::borrow::Cow;
use std::time::Instant;

use dot_api::{FileChange, PatchLine, Siblings, Text, Translator};
use dot_transcript::{
    CODE_LINES_KEPT, Change, ChangeHunk, ChangeLine, CodeWords, Neighbours, index, to_f32,
};
use winit::dpi::PhysicalPosition;

use crate::canvas::Layout;
use crate::conversation::Stage;
use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::tool::{revealed, revealed_within};
use crate::editor::Touched;

const CODE_COLUMNS: usize = 97;

pub(in crate::conversation::flow) struct Words<'a, T>(pub(in crate::conversation::flow) &'a T);

impl<T: Translator> CodeWords for Words<'_, T> {
    fn lines(&self, count: u64) -> Cow<'_, str> {
        self.0.count(Text::ToolLines, count)
    }

    fn new_file(&self) -> Cow<'_, str> {
        self.0.translate(Text::CodeNew)
    }

    fn fold_back(&self) -> Cow<'_, str> {
        self.0.translate(Text::CodeFold)
    }
}

impl Conversation {
    pub(crate) const fn touched(&self) -> &Touched {
        &self.touched
    }

    pub(crate) fn file_under(
        &self,
        at: PhysicalPosition<f64>,
        layout: &Layout,
        now: Instant,
    ) -> Option<String> {
        if self.question.is_some() || self.agent_view.is_some() {
            return None;
        }
        let stage = Stage::of(layout);
        let x = to_f32(at.x);
        if !(stage.column[0]..stage.column[1]).contains(&x) {
            return None;
        }
        let row = stage.row_at(to_f32(at.y), self.reader.focus(now))?;
        let column = index(f64::from(
            ((x - stage.column[0]) / layout.advance()).floor(),
        ));
        self.transcript.file_at(row, column, now).map(str::to_owned)
    }
}

pub(super) fn change_of(change: &FileChange) -> Change {
    let mut room = CODE_LINES_KEPT;
    let hunks = change
        .hunks
        .iter()
        .map(|hunk| ChangeHunk {
            new_start: hunk.new_start,
            lines: hunk
                .lines
                .iter()
                .map(|line| line_of(line, &mut room))
                .collect(),
        })
        .collect();
    Change {
        path: revealed(&change.path),
        created: change.created,
        hunks,
        neighbours: change.siblings.as_ref().map(neighbours_of),
    }
}

fn line_of(line: &PatchLine, room: &mut usize) -> ChangeLine {
    let shown = |text: &str, room: &mut usize| {
        if *room == 0 {
            return String::new();
        }
        *room -= 1;
        revealed_within(text, CODE_COLUMNS)
    };
    match line {
        PatchLine::Same(text) => ChangeLine::Same(shown(text, room)),
        PatchLine::Removed(text) => ChangeLine::Removed(shown(text, room)),
        PatchLine::Added(text) => ChangeLine::Added(shown(text, room)),
    }
}

fn neighbours_of(siblings: &Siblings) -> Neighbours {
    let names = |names: &[String]| names.iter().map(|name| revealed(name)).collect();
    Neighbours {
        folder: revealed(&siblings.folder),
        before: names(&siblings.before),
        after: names(&siblings.after),
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/reply/code.rs"]
mod tests;
