use std::time::Instant;

use dot_api::{
    Chat, Connector, Detail, ReplyEvent, ReplyFailure, Session, SessionStatus, ToolEnd, Translator,
};

use crate::app::DesktopApp;
use crate::canvas::{Canvas, Layout};
use crate::error::Problem;
use crate::event::Keystroke;
use crate::reply::{News, ReplyNews};
use crate::sign_in::Report;
use crate::worker::Mailbox;

impl<T, K, S, H, M, C> DesktopApp<'_, T, K, S, H, M, C>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
    M: Mailbox<Report> + Mailbox<SessionStatus> + Mailbox<ReplyNews>,
    C: Canvas,
{
    pub(super) fn hear(&mut self, news: ReplyNews, now: Instant) {
        let layout = self.layout();
        match self.replies.hear(news) {
            Some(News::Piece(event)) => {
                self.count_changes(&event);
                self.conversation.hear(event, &layout, now, self.translator);
            }
            Some(News::Asked(permission)) => {
                self.conversation
                    .ask(&permission, &layout, now, self.translator);
            }
            Some(News::Ended(Ok(reply))) => {
                self.status_bar.record(&reply);
                self.conversation.end_reply(None, &layout, now);
            }
            Some(News::Ended(Err(problem))) => {
                let notice = self.notice(&problem);
                self.conversation.end_reply(Some(&notice), &layout, now);
            }
            None => {}
        }
    }

    pub(super) fn answer(&mut self, keystroke: &Keystroke, layout: &Layout, now: Instant) {
        let Some(answer) = self
            .conversation
            .answer_to(&keystroke.key, keystroke.modifiers, now)
        else {
            return;
        };
        self.replies.answer(answer);
        self.conversation.close_question(layout);
    }

    pub(super) fn stop_reply(&mut self, now: Instant) {
        if !self.replies.stop() {
            return;
        }
        let notice = self.translator.translate(ReplyFailure::Stopped.text());
        let layout = self.layout();
        self.conversation.end_reply(Some(&notice), &layout, now);
    }

    fn count_changes(&mut self, event: &ReplyEvent) {
        if let ReplyEvent::ToolEnded {
            end: ToolEnd::Done(Detail::Changes { added, removed }),
            ..
        } = event
        {
            self.status_bar.count_changes(*added, *removed);
        }
    }

    fn notice(&self, problem: &Problem) -> String {
        let text = self.translator.translate(problem.text);
        match &problem.cause {
            Some(cause) => format!("{text}\n{cause}"),
            None => text.into_owned(),
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/answer.rs"]
mod tests;
