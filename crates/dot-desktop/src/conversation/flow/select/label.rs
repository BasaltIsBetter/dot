use std::time::Instant;

use dot_api::{Text, Translator};
use dot_transcript::{Ink, Line, Placed, Span, columns, rows, to_f32};

use crate::canvas::{Layout, Pane, TextSize};
use crate::conversation::Conversation;
use crate::conversation::flow::select::COPIED_SHOWN;
use crate::conversation::flow::select::draw::cooling_heat;
use crate::conversation::stage::Stage;
use crate::shell::COPY_KEY;

const LABEL_INK: Ink = Ink::Secondary;
const VALUE_INK: Ink = Ink::Full;
const FROM_THE_COLUMN: f32 = 24.0;
const LABEL_GAP: &str = "  ";

impl Conversation {
    pub(super) fn selection_label(
        &self,
        copied: Option<Instant>,
        (stage, layout): (&Stage, &Layout),
        top: f32,
        now: Instant,
        translator: &impl Translator,
    ) -> Pane {
        let (characters, lines) = self.selecting.as_ref().map_or((0, 0), |selecting| {
            selecting.selection.size(&self.transcript)
        });
        let copied = copied.filter(|at| COPIED_SHOWN.lasts(now.saturating_duration_since(*at)));
        let (copy, heat) = match copied {
            Some(at) => (
                translator.translate(Text::SelectionCopied),
                cooling_heat(at, now),
            ),
            None => (translator.translate(Text::SelectionCopy), 0),
        };
        let label = if lines > 1 {
            translator.with(Text::SelectionLines, &lines.to_string())
        } else {
            translator.translate(Text::SelectionLabel)
        };
        let count = translator.with(Text::SelectionCharacters, &characters.to_string());
        let labels = columns(&label).max(columns(COPY_KEY));
        let values = columns(&count).max(columns(&copy));
        let size_line = tag_line((&label, labels), Span::new(count.as_ref(), VALUE_INK));
        let copy_line = tag_line(
            (COPY_KEY, labels),
            Span::new(copy.as_ref(), VALUE_INK).heated(heat),
        );
        let small = layout.advance_of(TextSize::Small);
        let right = stage.column[0] - stage.points(FROM_THE_COLUMN);
        let widest = labels + columns(LABEL_GAP) + values;
        let left = right - to_f32(rows(widest)) * small;
        let placed = |line: Line, row_top: f32| Placed {
            top: layout.text_top(TextSize::Small, row_top, stage.row),
            line,
            clip: None,
        };
        Pane {
            left: left.round(),
            size: TextSize::Small,
            top: 0.0,
            bottom: stage.floor,
            lines: vec![placed(size_line, top), placed(copy_line, top + stage.row)],
        }
    }
}

fn tag_line((label, labels): (&str, usize), value: Span) -> Line {
    let indent = labels.saturating_sub(columns(label));
    Line::new(
        u16::try_from(indent).unwrap_or(u16::MAX),
        vec![
            Span::new(label, LABEL_INK),
            Span::new(LABEL_GAP, LABEL_INK),
            value,
        ],
    )
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/select/label.rs"]
mod tests;
