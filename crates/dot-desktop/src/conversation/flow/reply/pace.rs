use std::collections::VecDeque;
use std::mem;
use std::time::{Duration, Instant};

use dot_api::{Prompt, ReplyEvent, Translator};
use dot_transcript::{Cadence, columns, words_within};

use crate::conversation::flow::Conversation;

const LAGGING: Duration = Duration::from_millis(1_500);

#[derive(Debug, Default)]
pub(crate) struct Pacer {
    waiting: VecDeque<Waiting>,
    characters: usize,
    cadence: Cadence,
    held_until: Option<Instant>,
}

#[derive(Debug)]
pub(crate) struct Ended {
    pub(crate) next: Option<Prompt>,
}

#[derive(Debug)]
struct Waiting {
    since: Instant,
    step: Step,
}

#[derive(Debug)]
enum Step {
    Event(ReplyEvent),
    End {
        notice: Option<String>,
        written: Option<u64>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Release {
    Paced,
    Everything,
}

impl Pacer {
    pub(crate) fn is_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }

    fn push(&mut self, event: ReplyEvent, now: Instant) {
        self.characters += text_of(&event).map_or(0, columns);
        self.waiting.push_back(Waiting {
            since: now,
            step: Step::Event(event),
        });
    }

    fn push_end(&mut self, notice: Option<&str>, written: Option<u64>, now: Instant) {
        self.waiting.push_back(Waiting {
            since: now,
            step: Step::End {
                notice: notice.map(str::to_owned),
                written,
            },
        });
    }

    fn is_held(&self, now: Instant) -> bool {
        let lagging = self
            .waiting
            .front()
            .is_some_and(|waiting| now.saturating_duration_since(waiting.since) >= LAGGING);
        self.held_until.is_some_and(|until| now < until) && !lagging
    }

    fn next(
        &mut self,
        allowance: &mut usize,
        release: Release,
        now: Instant,
    ) -> Option<ReplyEvent> {
        if release == Release::Paced && self.is_held(now) {
            self.cadence.rest();
            return None;
        }
        let front = self.waiting.front_mut()?;
        let thinking = matches!(front.step, Step::Event(ReplyEvent::Thinking(_)));
        let piece = match &mut front.step {
            Step::End { .. } => return None,
            Step::Event(ReplyEvent::Text(piece) | ReplyEvent::Thinking(piece)) => piece,
            Step::Event(_) => return self.pop(),
        };
        let take = match release {
            Release::Everything => piece.len(),
            Release::Paced => words_within(piece, *allowance),
        };
        if take == 0 {
            return None;
        }
        let shown = columns(&piece[..take]);
        *allowance = allowance.saturating_sub(shown);
        self.characters = self.characters.saturating_sub(shown);
        self.cadence.spend(shown);
        if take == piece.len() {
            return self.pop();
        }
        let rest = piece.split_off(take);
        let shown = mem::replace(piece, rest);
        Some(if thinking {
            ReplyEvent::Thinking(shown)
        } else {
            ReplyEvent::Text(shown)
        })
    }

    fn pop(&mut self) -> Option<ReplyEvent> {
        match self.waiting.pop_front()?.step {
            Step::Event(event) => Some(event),
            Step::End { .. } => None,
        }
    }

    fn take_end(&mut self) -> Option<(Option<String>, Option<u64>)> {
        if !matches!(
            self.waiting.front(),
            Some(Waiting {
                step: Step::End { .. },
                ..
            })
        ) {
            return None;
        }
        match self.waiting.pop_front()?.step {
            Step::End { notice, written } => Some((notice, written)),
            Step::Event(_) => None,
        }
    }
}

impl Conversation {
    pub(crate) fn pace_replies(&mut self) {
        self.pacer.get_or_insert_with(Pacer::default);
    }

    pub(crate) fn is_pacing(&self) -> bool {
        self.pacer.as_ref().is_some_and(Pacer::is_waiting)
    }

    pub(crate) fn flow(&mut self, now: Instant, translator: &impl Translator) -> Option<Ended> {
        if self.pause.is_some() {
            return None;
        }
        self.show_paced(Release::Paced, now, translator);
        self.end_what_waited(now)
    }

    pub(crate) fn flush(&mut self, now: Instant, translator: &impl Translator) -> Option<Ended> {
        self.show_paced(Release::Everything, now, translator);
        self.end_what_waited(now)
    }

    pub(super) fn pace_in(
        &mut self,
        event: ReplyEvent,
        now: Instant,
        translator: &impl Translator,
    ) {
        let Some(pacer) = self.pacer.as_mut() else {
            self.apply(event, now, translator);
            return;
        };
        pacer.push(event, now);
        if self.pause.is_none() {
            self.show_paced(Release::Paced, now, translator);
        }
    }

    pub(super) fn wait_for_the_end(
        &mut self,
        notice: Option<&str>,
        written: Option<u64>,
        now: Instant,
    ) -> bool {
        let Some(pacer) = self.pacer.as_mut().filter(|pacer| pacer.is_waiting()) else {
            return false;
        };
        pacer.push_end(notice, written, now);
        true
    }

    pub(in crate::conversation::flow) fn show_what_waits(
        &mut self,
        now: Instant,
        translator: &impl Translator,
    ) -> bool {
        self.show_paced(Release::Everything, now, translator);
        let Some((notice, written)) = self.pacer.as_mut().and_then(Pacer::take_end) else {
            return false;
        };
        self.finish_reply(notice.as_deref(), written, now);
        true
    }

    fn show_paced(&mut self, release: Release, now: Instant, translator: &impl Translator) {
        let Some(pacer) = self.pacer.as_mut() else {
            return;
        };
        let mut allowance = match release {
            Release::Everything => usize::MAX,
            Release::Paced => pacer.cadence.allowance(pacer.characters, now),
        };
        loop {
            let Some(pacer) = self.pacer.as_mut() else {
                return;
            };
            let Some(event) = pacer.next(&mut allowance, release, now) else {
                if !pacer.is_waiting() {
                    pacer.cadence.rest();
                }
                return;
            };
            let draws_code = matches!(event, ReplyEvent::ToolChanged { .. });
            self.apply(event, now, translator);
            if draws_code && let Some(pacer) = self.pacer.as_mut() {
                pacer.held_until = self.transcript.grows_until();
            }
        }
    }

    fn end_what_waited(&mut self, now: Instant) -> Option<Ended> {
        let (notice, written) = self.pacer.as_mut()?.take_end()?;
        self.finish_reply(notice.as_deref(), written, now);
        Some(Ended {
            next: self.send_the_queue(now),
        })
    }
}

fn text_of(event: &ReplyEvent) -> Option<&str> {
    match event {
        ReplyEvent::Text(piece) | ReplyEvent::Thinking(piece) => Some(piece),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/reply/pace.rs"]
mod tests;
