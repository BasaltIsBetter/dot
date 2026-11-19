use std::time::Instant;

use dot_api::{Permission, ReplyEvent, Translator};
use winit::keyboard::{Key, ModifiersState};

use crate::canvas::Layout;
use crate::conversation::flow::Conversation;
use crate::conversation::flow::question::Question;
use crate::conversation::flow::tool::{ended, started};
use crate::conversation::transcript::Kind;
use crate::reply::Answer;

impl Conversation {
    pub(crate) fn hear(
        &mut self,
        event: ReplyEvent,
        layout: &Layout,
        now: Instant,
        translator: &impl Translator,
    ) {
        let visible = self.visible_rows(layout);
        match event {
            ReplyEvent::Thinking(piece) => {
                self.transcript.hear(Kind::Thinking, &piece, visible, now);
            }
            ReplyEvent::Text(piece) => self.transcript.hear(Kind::Answer, &piece, visible, now),
            ReplyEvent::ToolStarted { id, tool } => {
                let shown = started(&tool, translator);
                self.transcript.start_tool(id, &shown, visible, now);
            }
            ReplyEvent::ToolEnded { id, end } => {
                let detail = ended(end, translator);
                self.transcript.end_tool(&id, &detail, visible, now);
            }
        }
    }

    pub(crate) fn end_reply(&mut self, notice: Option<&str>, layout: &Layout, now: Instant) {
        self.close_question(layout);
        let visible = self.visible_rows(layout);
        self.transcript.end_reply(notice, visible, now);
    }

    pub(crate) fn ask(
        &mut self,
        permission: &Permission,
        layout: &Layout,
        now: Instant,
        translator: &impl Translator,
    ) {
        if !self.transcript.is_live() {
            return;
        }
        let width = self.prompter.width();
        self.question = Some(Question::new(permission, width, now, translator));
        self.refit_transcript(layout);
    }

    pub(crate) fn answer_to(
        &self,
        key: &Key,
        modifiers: ModifiersState,
        now: Instant,
    ) -> Option<Answer> {
        self.question.as_ref()?.answer_to(key, modifiers, now)
    }

    pub(crate) fn close_question(&mut self, layout: &Layout) {
        if self.question.take().is_some() {
            self.refit_transcript(layout);
        }
    }

    pub(crate) const fn is_asking(&self) -> bool {
        self.question.is_some()
    }

    fn refit_transcript(&mut self, layout: &Layout) {
        let visible = self.visible_rows(layout);
        self.transcript.refit(visible);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/reply.rs"]
mod tests;
