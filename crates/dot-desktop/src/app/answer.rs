use std::time::Instant;

use dot_api::{ReplyFailure, Text, Translator};

use crate::app::{DesktopApp, Parts};
use crate::conversation::Command;
use crate::event::Keystroke;
use crate::reply::{News, ReplyNews};
use crate::sessions::SessionKey;

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn hear(&mut self, key: SessionKey, news: ReplyNews, now: Instant) {
        let translator = self.translator;
        let Some((active, opened)) = self.sessions.by_key_mut(key) else {
            return;
        };
        let Some(news) = opened.replies.hear(news) else {
            return;
        };
        let conversation = &mut opened.conversation;
        let ended = match news {
            News::Piece(event) => {
                conversation.hear(event, now, translator);
                return;
            }
            News::Asked(permission) => {
                conversation.ask(&permission, now, translator);
                return;
            }
            News::Ended(Ok(reply)) => conversation.end_reply(None, Some(reply.output_tokens), now),
            News::Ended(Err(problem)) => {
                tracing::warn!(
                    failure = problem.text.id(),
                    cause = problem.cause.as_deref().unwrap_or_default(),
                    "a reply failed"
                );
                let notice = problem.notice(translator);
                conversation.end_reply(Some(&notice), None, now)
            }
        };
        if let Some(prompt) = ended {
            opened.ask(prompt, &mut self.recall);
        }
        if !active && !opened.conversation.is_live() {
            opened.unread = Some(now);
        }
    }

    pub(super) fn run(&mut self, command: Command, now: Instant) {
        let opened = self.sessions.active_mut();
        match command {
            Command::Ask(prompt) => opened.ask(prompt, &mut self.recall),
            Command::Redirect(prompt) => opened.redirect(prompt, &mut self.recall),
            Command::Stop => self.stop_reply(now),
            Command::StartOver => self.start_over(now),
        }
    }

    pub(super) fn answer(&mut self, keystroke: &Keystroke, now: Instant) {
        let opened = self.sessions.active_mut();
        let conversation = &mut opened.conversation;
        let Some(answer) = conversation.answer_to(&keystroke.key, keystroke.modifiers, now) else {
            return;
        };
        opened.replies.answer(answer);
        conversation.close_question();
    }

    pub(super) fn stop_reply(&mut self, now: Instant) {
        let translator = self.translator;
        let opened = self.sessions.active_mut();
        let shown = opened.conversation.flush(now, translator);
        if let Some(prompt) = shown.and_then(|ended| ended.next) {
            opened.ask(prompt, &mut self.recall);
        }
        if !opened.replies.stop() {
            return;
        }
        let notice = translator.translate(ReplyFailure::Stopped.text());
        if let Some(prompt) = opened.conversation.end_reply(Some(&notice), None, now) {
            opened.ask(prompt, &mut self.recall);
        }
    }

    pub(super) fn start_over(&mut self, now: Instant) {
        let notice = self.translator.translate(Text::ConversationRestarted);
        let opened = self.sessions.active_mut();
        opened.conversation.flush(now, self.translator);
        opened.replies.start_over();
        opened.conversation.notice(&notice, now);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/answer.rs"]
mod tests;
