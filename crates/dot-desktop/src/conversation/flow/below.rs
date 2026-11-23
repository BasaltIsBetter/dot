use std::time::Instant;

use dot_api::{Text, Translator};

use crate::conversation::blink::Caret;
use crate::conversation::flow::Conversation;
use crate::conversation::flow::pause::Queued;
use crate::conversation::flow::question::Question;
use crate::conversation::prompter::Mark;
use crate::conversation::stage::Stage;
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
    pub(super) fn plan(&self, stage: &Stage) -> Plan {
        let below = self.below();
        let Some(anchor) = self.transcript.anchor() else {
            return Plan {
                resting: self.caret_row(),
                last: below.last,
                writing: None,
            };
        };
        let writing = match (below.note, below.running) {
            (Some(_), _) => Some(self.caret_row()),
            (None, Some(running)) => Some(running),
            (None, None) => self.transcript.writing_row(),
        };
        Plan {
            resting: anchor.max(below.last.saturating_sub(stage.rows_below())),
            last: below.last,
            writing,
        }
    }

    pub(super) fn below(&self) -> Below {
        let end = self.transcript.end_row();
        let running_rows = self.running_rows().len();
        let running = (running_rows > 0).then_some(end);
        let after = end + running_rows;
        let (note, mut next) = match &self.pause {
            Some(_) => (Some(after), after + self.prompter.rows().max(1) + 1),
            None => (None, self.first_row_below(after)),
        };
        let queued = self.queued.as_ref().map(|queued| {
            let at = next;
            next += queued_rows(queued, self.prompter.width()) + 1;
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
        let below = self.below();
        let top_of = |first: usize| move |row: usize| stage.row_top(first + row, top);
        if let Some(question) = &self.question {
            return question.lines(top_of(below.draft));
        }
        let mut placed = match (below.note, &self.pause) {
            (Some(note), Some(pause)) => {
                let mut placed = self.note_lines((top_of(note), stage.row), now, translator);
                placed.extend(pause.draft.lines(
                    (top_of(below.draft), stage.row),
                    (Caret::Dark, Mark::Draft),
                    None,
                    now,
                ));
                placed
            }
            _ => self.draft_lines((top_of(below.draft), stage.row), now, translator),
        };
        if let (Some(at), Some(queued)) = (below.queued, &self.queued) {
            let width = self.prompter.width();
            placed.extend(queued_lines(queued, width, top_of(at), translator));
        }
        placed
    }

    fn note_lines(
        &self,
        rows: (impl Fn(usize) -> f32, f32),
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
        rows: (impl Fn(usize) -> f32, f32),
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

    pub(super) fn caret_row(&self) -> usize {
        let below = self.below();
        let (row, _) = self.prompter.caret_cell();
        below.note.unwrap_or(below.draft) + row
    }

    pub(super) fn editing_row(&self) -> usize {
        let below = self.below();
        below.note.unwrap_or(below.draft)
    }

    fn rows_below(&self) -> usize {
        self.question
            .as_ref()
            .map_or_else(|| self.prompter.rows(), Question::rows)
    }
}

fn queued_rows(queued: &Queued, width: TextWidth) -> usize {
    wrap(&queued.text, width.columns()).len() + queued.attachments.len()
}

fn queued_lines(
    queued: &Queued,
    width: TextWidth,
    top_of: impl Fn(usize) -> f32,
    translator: &impl Translator,
) -> Vec<Placed> {
    let rows = wrap(&queued.text, width.columns());
    let mut lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(index, range)| {
            let text = Span::new(&queued.text[range.clone()], QUEUED_INK);
            if index == 0 {
                Line::new(0, vec![Span::new(MARKER, QUEUED_INK), text])
            } else {
                Line::new(TEXT_INDENT, vec![text])
            }
        })
        .collect();
    lines.extend(queued.attachments.iter().map(|attachment| {
        Line::new(
            TEXT_INDENT,
            vec![Span::new(attachment.label(width), QUEUED_INK)],
        )
    }));
    if let Some(last) = lines.last_mut() {
        let after = translator.translate(Text::QueuedAfterTheReply);
        last.spans.push(Span::new(
            format!("{QUEUED_SEPARATOR}{after}"),
            QUEUED_SUFFIX_INK,
        ));
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(row, line)| Placed {
            top: top_of(row),
            line,
            clip: None,
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/below.rs"]
mod tests;
