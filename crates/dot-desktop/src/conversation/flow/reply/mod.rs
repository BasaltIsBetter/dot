mod agent;
mod code;
mod pace;
mod tool;

use std::time::Instant;

use dot_api::{Permission, Prompt, ReplyEvent, Text, ToolKind, Translator};
use winit::keyboard::{Key, ModifiersState};

use crate::conversation::flow::Conversation;
use crate::conversation::flow::question::Question;
use crate::conversation::flow::reply::code::change_of;
use crate::conversation::flow::reply::tool::{Summed, aside, result, started};

pub(in crate::conversation) use self::agent::{name_of, summary};
pub(in crate::conversation::flow) use self::code::Words;

pub(crate) use self::pace::Pacer;
pub(crate) use self::tool::revealed;
use crate::reply::Answer;
use crate::status_bar::token_count;
use dot_transcript::Kind;

impl Conversation {
    pub(crate) fn hear(&mut self, event: ReplyEvent, now: Instant, translator: &impl Translator) {
        if let ReplyEvent::Thinking(piece) | ReplyEvent::Text(piece) = &event {
            self.instrument.hear(piece.chars().count(), now);
        }
        self.count_agent(&event, now);
        if let Some(event) = self.hold(event) {
            self.pace_in(event, now, translator);
        }
    }

    pub(super) fn apply(&mut self, event: ReplyEvent, now: Instant, translator: &impl Translator) {
        match event {
            ReplyEvent::Thinking(piece) => self.transcript.hear(Kind::Thinking, &piece, now),
            ReplyEvent::Text(piece) => {
                self.writing = true;
                self.transcript.hear(Kind::Answer, &piece, now);
            }
            ReplyEvent::ToolStarted { id, tool } => {
                if tool.kind == ToolKind::Delegate && self.transcript.is_live() {
                    self.agents.start(id.clone(), tool.subject.clone(), now);
                }
                let tool = started(&tool, translator);
                self.transcript.start_tool(id, tool, now);
            }
            ReplyEvent::ToolOutput { id, lines } => self.transcript.tool_output(&id, lines),
            ReplyEvent::ToolChanged { id, change } => {
                self.transcript.change_tool(&id, change_of(&change), now);
                self.touched.record(*change);
            }
            ReplyEvent::ToolEnded { id, end } => {
                let result = result(end, translator);
                self.agents.end(&id, now);
                let aside = match self.agents.get(&id) {
                    Some(agent) => Some(Summed::whole(agent::aside(agent, now, translator))),
                    None => self.transcript.running_tool(&id).and_then(|(tool, since)| {
                        let elapsed = now.saturating_duration_since(since);
                        aside(tool, (&result, elapsed), translator)
                    }),
                };
                let aside = aside.as_ref().map(Summed::as_aside);
                self.transcript.end_tool(&id, result, aside, now);
            }
            ReplyEvent::Agent { id, event } => self.follow_agent(&id, event, translator),
            ReplyEvent::Compacted { before, after } => {
                let notice = compacted(before, after, translator);
                self.transcript.hear(Kind::Notice, &notice, now);
            }
        }
    }

    pub(crate) fn end_reply(
        &mut self,
        notice: Option<&str>,
        written: Option<u64>,
        now: Instant,
    ) -> Option<Prompt> {
        if self.hold_the_end(notice, written) || self.wait_for_the_end(notice, written, now) {
            return None;
        }
        self.finish_reply(notice, written, now);
        self.send_the_queue(now)
    }

    pub(super) fn end_without_a_word(&mut self, now: Instant) {
        self.finish_reply(None, None, now);
    }

    pub(super) fn finish_reply(
        &mut self,
        notice: Option<&str>,
        written: Option<u64>,
        now: Instant,
    ) {
        self.question = None;
        self.agents.end_working(now);
        self.instrument.end(written, now);
        self.transcript.end_reply(notice, now);
        self.help.stir(now);
    }

    pub(crate) fn ask(
        &mut self,
        permission: &Permission,
        now: Instant,
        translator: &impl Translator,
    ) {
        if !self.transcript.is_live() {
            return;
        }
        let width = self.prompter.width();
        self.question = Some(Question::new(permission, width, now, translator));
    }

    pub(crate) fn answer_to(
        &self,
        key: &Key,
        modifiers: ModifiersState,
        now: Instant,
    ) -> Option<Answer> {
        self.question.as_ref()?.answer_to(key, modifiers, now)
    }

    pub(crate) fn close_question(&mut self) {
        self.question = None;
    }

    pub(crate) const fn is_asking(&self) -> bool {
        self.question.is_some()
    }
}

fn compacted(before: u64, after: u64, translator: &impl Translator) -> String {
    format!(
        "{} {} → {}",
        translator.translate(Text::ConversationCompacted),
        token_count(before),
        token_count(after)
    )
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/reply/mod.rs"]
mod tests;
