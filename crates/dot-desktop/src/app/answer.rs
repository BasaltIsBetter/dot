use std::time::Instant;

use dot_api::{ReplyFailure, Text, Translator};

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::Layout;
use crate::error::Problem;
use crate::event::Keystroke;
use crate::reply::{News, ReplyNews};

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn hear(&mut self, news: ReplyNews, now: Instant) {
        let layout = self.layout();
        let Some(news) = self.replies.hear(news) else {
            return;
        };
        let Screen::Chat(conversation) = &mut self.screen else {
            return;
        };
        match news {
            News::Piece(event) => conversation.hear(event, &layout, now, self.translator),
            News::Asked(permission) => {
                conversation.ask(&permission, &layout, now, self.translator);
            }
            News::Ended(Ok(_)) => conversation.end_reply(None, &layout, now),
            News::Ended(Err(problem)) => {
                tracing::warn!(
                    failure = problem.text.id(),
                    cause = problem.cause.as_deref().unwrap_or_default(),
                    "a reply failed"
                );
                let notice = notice(self.translator, &problem);
                conversation.end_reply(Some(&notice), &layout, now);
            }
        }
    }

    pub(super) fn answer(&mut self, keystroke: &Keystroke, layout: &Layout, now: Instant) {
        let Screen::Chat(conversation) = &mut self.screen else {
            return;
        };
        let Some(answer) = conversation.answer_to(&keystroke.key, keystroke.modifiers, now) else {
            return;
        };
        self.replies.answer(answer);
        conversation.close_question(layout);
    }

    pub(super) fn stop_reply(&mut self, now: Instant) {
        if !self.replies.stop() {
            return;
        }
        let layout = self.layout();
        let Screen::Chat(conversation) = &mut self.screen else {
            return;
        };
        let notice = self.translator.translate(ReplyFailure::Stopped.text());
        conversation.end_reply(Some(&notice), &layout, now);
    }

    pub(super) fn start_over(&mut self, now: Instant) {
        let Screen::Chat(conversation) = &mut self.screen else {
            return;
        };
        self.replies.start_over();
        let notice = self.translator.translate(Text::ConversationRestarted);
        conversation.notice(&notice, now);
    }
}

fn notice(translator: &impl Translator, problem: &Problem) -> String {
    let text = translator.translate(problem.text);
    match &problem.cause {
        Some(cause) => format!("{text}\n{cause}"),
        None => text.into_owned(),
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/answer.rs"]
mod tests;
