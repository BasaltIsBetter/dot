use dot_api::{Text, Translator};
use dot_transcript::{Ink, Line, Placed, Span, columns, index, rows, to_f32, truncated};

use crate::canvas::{Layout, Pane, TextSize};
use crate::editor::versions::Toward;
use crate::editor::view::Frame;
use crate::editor::{Editor, Saving};

const BAR_LEFT: f32 = 24.0;
const TREE_TOP: f32 = 56.0;
const TREE_LEFT: f32 = 24.0;
const TREE_RIGHT: f32 = 16.0;
const GAP: usize = 2;
const BACK: &str = "↶";
const FORWARD: &str = "↷";
const SEPARATOR: &str = " · ";
const BETWEEN: &str = "  ";
const SPACE: &str = " ";
const FOLDER_MARK: char = '/';
const ROOT: &str = "./";
const EDITED: &str = "✎ ";
const CREATED: &str = "✚ ";
const UNMARKED: &str = "  ";
const NESTED: u16 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Hit {
    Back,
    Forward,
    Close,
    Open(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Bar {
    left: f32,
    advance: f32,
    columns: usize,
    back: usize,
    forward: usize,
    close: usize,
    close_text: (String, String),
    tree: Option<[f32; 3]>,
}

impl Bar {
    pub(super) fn of(frame: &Frame, layout: &Layout, translator: &impl Translator) -> Self {
        let left = frame.pane + layout.scaled(BAR_LEFT);
        let advance = layout.advance_of(TextSize::Small);
        let columns = index(f64::from(((frame.right - left) / advance).floor().max(0.0)));
        let key = translator.translate(Text::HelpStopKey).into_owned();
        let action = translator.translate(Text::EditorBack).into_owned();
        let close = columns.saturating_sub(columns_of(&key) + 1 + columns_of(&action));
        let forward = close.saturating_sub(GAP + 1);
        let back = forward.saturating_sub(GAP + 1);
        let tree = frame.tree.map(|right| {
            [
                layout.scaled(TREE_LEFT),
                layout.scaled(TREE_TOP),
                right - layout.scaled(TREE_RIGHT),
            ]
        });
        Self {
            left,
            advance,
            columns,
            back,
            forward,
            close,
            close_text: (key, action),
            tree,
        }
    }

    pub(super) fn hit(&self, editor: &Editor, [x, y]: [f32; 2], frame: &Frame) -> Option<Hit> {
        if (frame.bar_top..frame.code_top).contains(&y) && x >= self.left {
            let column = index(f64::from(((x - self.left) / self.advance).floor()));
            return match column {
                at if at == self.back => Some(Hit::Back),
                at if at == self.forward => Some(Hit::Forward),
                at if at >= self.close && at < self.columns => Some(Hit::Close),
                _ => None,
            };
        }
        let [left, top, right] = self.tree?;
        if !(left..right).contains(&x) || y < top {
            return None;
        }
        let row = index(f64::from(((y - top) / frame.row).floor()));
        let entry = editor.entries.get(row.checked_sub(1)?)?;
        (entry.touched.is_some() && entry.path != editor.path)
            .then(|| Hit::Open(entry.path.clone()))
    }

    pub(super) fn pane(
        &self,
        editor: &Editor,
        frame: &Frame,
        pointer: Option<[f32; 2]>,
        translator: &impl Translator,
    ) -> Pane {
        let hovered = pointer.and_then(|at| self.hit(editor, at, frame));
        let current = editor.buffer.text();
        let back = editor.versions.back(current);
        let forward = editor.versions.forward(current);
        let mut spans = Self::path(editor);
        spans.push(Span::new(BETWEEN, Ink::Secondary));
        spans.push(Span::new(Self::counts(editor, translator), Ink::Secondary));
        let hint = match hovered {
            Some(Hit::Back) => back,
            Some(Hit::Forward) => forward,
            _ => None,
        };
        if let Some(toward) = hint {
            spans.push(Span::new(
                format!("{BETWEEN}{}", translator.translate(text_of(toward))),
                Ink::Secondary,
            ));
        }
        let room = self.back.saturating_sub(GAP);
        let left = fit(spans, room);
        let arrow = |text: &str, at: usize, toward: Option<Toward>, hit: Hit| {
            let ink = match (toward, hovered == Some(hit)) {
                (None, _) => Ink::Faint,
                (Some(_), true) => Ink::Full,
                (Some(_), false) => Ink::Secondary,
            };
            line_at(at, vec![Span::new(text, ink)])
        };
        let (key, action) = &self.close_text;
        let lines = [
            Line::new(0, left),
            arrow(BACK, self.back, back, Hit::Back),
            arrow(FORWARD, self.forward, forward, Hit::Forward),
            line_at(
                self.close,
                vec![
                    Span::new(key.as_str(), Ink::Full),
                    Span::new(format!("{SPACE}{action}"), Ink::Secondary),
                ],
            ),
        ];
        let top = frame.bar_top;
        Pane {
            left: self.left,
            size: TextSize::Small,
            top: 0.0,
            bottom: frame.code_top,
            lines: lines
                .into_iter()
                .map(|line| Placed {
                    top,
                    line,
                    clip: None,
                })
                .collect(),
        }
    }

    pub(super) fn tree(&self, editor: &Editor, layout: &Layout, frame: &Frame) -> Option<Pane> {
        let [left, top, right] = self.tree?;
        let room = index(f64::from(((right - left) / self.advance).floor().max(0.0)));
        let folder = editor
            .folder
            .rsplit(FOLDER_MARK)
            .next()
            .filter(|name| !name.is_empty())
            .map_or_else(|| ROOT.to_owned(), |name| format!("{name}{FOLDER_MARK}"));
        let mut lines = vec![Line::new(
            0,
            vec![Span::new(truncated(&folder, room), Ink::Receding)],
        )];
        for entry in &editor.entries {
            let (mark, ink) = match entry.touched {
                Some(created) if entry.path == editor.path => (mark_of(created), Ink::Full),
                Some(created) => (mark_of(created), Ink::Secondary),
                None => (UNMARKED, Ink::Receding),
            };
            let name = truncated(
                &entry.name,
                room.saturating_sub(usize::from(NESTED) + columns_of(mark)),
            );
            lines.push(Line::new(
                NESTED,
                vec![Span::new(mark, Ink::Full), Span::new(name, ink)],
            ));
        }
        let shown = index(f64::from(
            ((frame.bottom - top) / frame.row).floor().max(0.0),
        ));
        let lines = lines
            .into_iter()
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

    fn path(editor: &Editor) -> Vec<Span> {
        let (folder, name) = editor
            .path
            .rsplit_once(FOLDER_MARK)
            .map_or(("", editor.path.as_str()), |(folder, name)| (folder, name));
        let mut spans = Vec::new();
        if !folder.is_empty() {
            spans.push(Span::new(format!("{folder}{FOLDER_MARK}"), Ink::Receding));
        }
        spans.push(Span::new(name, Ink::Full));
        spans
    }

    fn counts(editor: &Editor, translator: &impl Translator) -> String {
        let claude = editor.versions.claude_count();
        let lines = |count: usize| {
            let count = u64::try_from(count).unwrap_or(u64::MAX);
            format!("{count} {}", translator.count(Text::ToolLines, count))
        };
        let by_claude = if editor.versions.created() {
            Text::EditorCreatedByClaude
        } else {
            Text::EditorByClaude
        };
        let counts = format!("{} {}", translator.translate(by_claude), lines(claude));
        let mine = editor.mine.count();
        if mine == 0 {
            return counts;
        }
        let yours = translator.translate(Text::EditorByYou);
        format!("{counts}{SEPARATOR}{yours} {}", lines(mine))
    }
}

pub(crate) fn activity(editor: &Editor, translator: &impl Translator) -> String {
    let name = editor
        .path
        .rsplit(FOLDER_MARK)
        .next()
        .unwrap_or(&editor.path);
    let state = match editor.saving() {
        Saving::Saved => translator.translate(Text::EditorSaved),
        Saving::Failed(_) => translator.translate(Text::EditorNotSaved),
    };
    format!("{name}{SEPARATOR}{state}")
}

const fn text_of(toward: Toward) -> Text {
    match toward {
        Toward::Claude => Text::EditorToClaude,
        Toward::Before => Text::EditorToBefore,
        Toward::Yours => Text::EditorToYours,
    }
}

const fn mark_of(created: bool) -> &'static str {
    if created { CREATED } else { EDITED }
}

fn fit(spans: Vec<Span>, room: usize) -> Vec<Span> {
    let mut left = room;
    spans
        .into_iter()
        .filter_map(|span| {
            let kept = truncated(&span.text, left);
            left = left.saturating_sub(columns_of(&kept));
            (!kept.is_empty()).then_some(Span { text: kept, ..span })
        })
        .collect()
}

fn line_at(column: usize, spans: Vec<Span>) -> Line {
    Line::new(u16::try_from(column).unwrap_or(u16::MAX), spans)
}

fn columns_of(text: &str) -> usize {
    columns(text)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/view/bar.rs"]
mod tests;
