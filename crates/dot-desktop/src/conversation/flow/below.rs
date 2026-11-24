use std::time::Instant;

use dot_api::{Text, Translator};

use crate::conversation::blink::Caret;
use crate::conversation::flow::Conversation;
use crate::conversation::flow::pause::Queued;
use crate::conversation::flow::question::Question;
use crate::conversation::prompter::Mark;
use crate::conversation::stage::{Stage, rows_in_sight};
use crate::conversation::suggestion::Hint;
use dot_transcript::{Ink, Line, MARKER, Placed, Plan, Span, TEXT_INDENT, TextWidth, wrap};

const QUEUED_INK: Ink = Ink::Secondary;
const QUEUED_SUFFIX_INK: Ink = Ink::Receding;
const QUEUED_SEPARATOR: &str = " · ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Below {
    pub(super) running: Option<usize>,
    pub(super) note: Option<usize>,
    pub(super) queued: Option<usize>,
    pub(super) draft: usize,
    pub(super) last: usize,
}

impl Conversation {
    pub(super) fn plan(&self, stage: &Stage, now: Instant) -> Plan {
        let below = self.below(now);
        let Some(anchor) = self.transcript.anchor() else {
            return Plan {
                resting: self.caret_row(now),
                last: below.last,
                writing: None,
            };
        };
        let writing = match (below.note, below.running) {
            (Some(_), _) => Some(self.caret_row(now)),
            (None, Some(running)) => Some(running),
            (None, None) => self.transcript.writing_row(),
        };
        Plan {
            resting: anchor.max(below.last.saturating_sub(stage.rows_below())),
            last: below.last,
            writing,
        }
    }

    pub(super) fn below(&self, now: Instant) -> Below {
        let end = self.transcript.end_row();
        let running_rows = self.running_rows(now).len();
        let running = (running_rows > 0).then_some(end);
        let after = end + running_rows;
        let (note, mut next) = match &self.pause {
            Some(_) => (Some(after), after + self.prompter.rows().max(1) + 1),
            None => (None, self.first_row_below(after)),
        };
        let queued = self.queued.as_ref().map(|queued| {
            let at = next;
            next += queued.rows_at(self.prompter.width()) + 1;
            at
        });
        let draft_rows = self
            .pause
            .as_ref()
            .map_or_else(|| self.rows_below(), |pause| pause.draft.rows());
        Below {
            running,
            note,
            queued,
            draft: next,
            last: next + draft_rows.max(1) - 1,
        }
    }

    fn first_row_below(&self, end: usize) -> usize {
        let gap = self.transcript.is_live() || end > 0;
        end + usize::from(gap)
    }

    pub(super) fn below_lines(
        &self,
        stage: &Stage,
        top: f64,
        now: Instant,
        translator: &impl Translator,
    ) -> Vec<Placed> {
        let below = self.below(now);
        let top_of = |first: usize| move |row: usize| stage.row_top(first + row, top);
        if let Some(question) = &self.question {
            return question.lines(top_of(below.draft), (stage.row, stage.height));
        }
        let mut placed = match (below.note, &self.pause) {
            (Some(note), Some(pause)) => {
                let mut placed =
                    self.note_lines((top_of(note), stage.row, stage.height), now, translator);
                placed.extend(pause.draft.lines(
                    (top_of(below.draft), stage.row, stage.height),
                    (Caret::Dark, Mark::Draft),
                    None,
                    now,
                ));
                placed
            }
            _ => self.draft_lines(
                (top_of(below.draft), stage.row, stage.height),
                now,
                translator,
            ),
        };
        if let (Some(at), Some(queued)) = (below.queued, &self.queued) {
            let width = self.prompter.width();
            let sight = (top_of(at), stage.row, stage.height);
            placed.extend(queued_lines(queued, width, sight, translator));
        }
        placed
    }

    fn note_lines(
        &self,
        rows: (impl Fn(usize) -> f32, f32, f32),
        now: Instant,
        translator: &impl Translator,
    ) -> Vec<Placed> {
        let placeholder = Hint::still(translator.translate(Text::NotePlaceholder).into_owned());
        let caret = if self.prompter.is_empty() {
            Caret::Dark
        } else {
            self.blink.caret(now)
        };
        self.prompter
            .lines(rows, (caret, Mark::Note), Some(&placeholder), now)
    }

    fn draft_lines(
        &self,
        rows: (impl Fn(usize) -> f32, f32, f32),
        now: Instant,
        translator: &impl Translator,
    ) -> Vec<Placed> {
        let live = self.transcript.is_live();
        let hint =
            (!live && self.prompter.is_empty()).then(|| self.suggestions.hint(now, translator));
        let caret = if live && self.prompter.is_empty() {
            Caret::Dark
        } else {
            self.blink.caret(now)
        };
        self.prompter
            .lines(rows, (caret, Mark::Draft), hint.as_ref(), now)
    }

    pub(super) fn caret_row(&self, now: Instant) -> usize {
        let below = self.below(now);
        let (row, _) = self.prompter.caret_cell();
        below.note.unwrap_or(below.draft) + row
    }

    pub(super) fn editing_row(&self, now: Instant) -> usize {
        let below = self.below(now);
        below.note.unwrap_or(below.draft)
    }

    fn rows_below(&self) -> usize {
        self.question
            .as_ref()
            .map_or_else(|| self.prompter.rows(), Question::rows)
    }
}

fn queued_lines(
    queued: &Queued,
    width: TextWidth,
    (top_of, row, height): (impl Fn(usize) -> f32, f32, f32),
    translator: &impl Translator,
) -> Vec<Placed> {
    let rows = wrap(&queued.text, width.columns());
    let count = rows.len() + queued.attachments.len();
    rows_in_sight(count, &top_of, row, height)
        .filter_map(|index| {
            let mut line = if let Some(range) = rows.get(index) {
                let text = Span::new(&queued.text[range.clone()], QUEUED_INK);
                if index == 0 {
                    Line::new(0, vec![Span::new(MARKER, QUEUED_INK), text])
                } else {
                    Line::new(TEXT_INDENT, vec![text])
                }
            } else {
                let attachment = queued.attachments.get(index - rows.len())?;
                Line::new(
                    TEXT_INDENT,
                    vec![Span::new(attachment.label(width), QUEUED_INK)],
                )
            };
            if index + 1 == count {
                let after = translator.translate(Text::QueuedAfterTheReply);
                line.spans.push(Span::new(
                    format!("{QUEUED_SEPARATOR}{after}"),
                    QUEUED_SUFFIX_INK,
                ));
            }
            Some(Placed {
                top: top_of(index),
                line,
                clip: None,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/below.rs"]
mod tests;
