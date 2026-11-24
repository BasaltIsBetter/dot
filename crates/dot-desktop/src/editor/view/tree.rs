use std::iter;

use dot_transcript::{Ink, Line, Placed, Span, columns, index, rows, to_f32, truncated};

use crate::canvas::{Layout, Pane, TextSize};
use crate::conversation::revealed;
use crate::editor::view::Frame;
use crate::editor::view::bar::{Bar, FOLDER_MARK, Hit};
use crate::editor::{Editor, Entry};

const ROOT: &str = "./";
const EDITED: &str = "✎ ";
const CREATED: &str = "✚ ";
const UNMARKED: &str = "  ";
const NESTED: u16 = 2;

impl Bar {
    pub(super) fn tree(
        &self,
        editor: &Editor,
        layout: &Layout,
        frame: &Frame,
        hovered: Option<&Hit>,
    ) -> Option<Pane> {
        let [left, top, right] = self.tree?;
        let room = index(f64::from(((right - left) / self.advance).floor().max(0.0)));
        let folder = editor
            .folder
            .rsplit(FOLDER_MARK)
            .next()
            .filter(|name| !name.is_empty())
            .map_or_else(
                || ROOT.to_owned(),
                |name| format!("{}{FOLDER_MARK}", revealed(name)),
            );
        let shown = index(f64::from(
            ((frame.bottom - top) / frame.row).floor().max(0.0),
        ));
        let header = Line::new(0, vec![Span::new(truncated(&folder, room), Ink::Receding)]);
        let entries = editor.entries.iter().map(|entry| {
            let lit = matches!(hovered, Some(Hit::Open(path)) if *path == entry.path);
            entry_line(entry, (editor, lit), room)
        });
        let lines = iter::once(header)
            .chain(entries)
            .take(shown)
            .enumerate()
            .map(|(row, line)| {
                let row_top = (top + to_f32(rows(row)) * frame.row).round();
                Placed {
                    top: layout.text_top(TextSize::Small, row_top, frame.row),
                    line,
                    clip: None,
                }
            });
        Some(Pane {
            left,
            size: TextSize::Small,
            top: 0.0,
            bottom: frame.bottom,
            lines: lines.collect(),
        })
    }
}

const fn mark_of(created: bool) -> &'static str {
    if created { CREATED } else { EDITED }
}

fn entry_line(entry: &Entry, (editor, lit): (&Editor, bool), room: usize) -> Line {
    let (mark, ink) = match entry.touched {
        Some(created) if entry.path == editor.path || lit => (mark_of(created), Ink::Full),
        Some(created) => (mark_of(created), Ink::Secondary),
        None => (UNMARKED, Ink::Receding),
    };
    let name = truncated(
        &revealed(&entry.name),
        room.saturating_sub(usize::from(NESTED) + columns(mark)),
    );
    Line::new(
        NESTED,
        vec![Span::new(mark, Ink::Full), Span::new(name, ink)],
    )
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/view/tree.rs"]
mod tests;
