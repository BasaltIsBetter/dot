mod asking;
mod runner;

use std::sync::Arc;

use dot_api::{
    Cancellation, Chat, Connector, Permission, PermissionMode, Prompt, Reply, ReplyEvent, Session,
};

use crate::error::Problem;
use crate::worker::Mailbox;

pub(crate) use self::asking::Answer;
use self::asking::Asking;
use self::runner::{Crew, Job, Runner};

pub(crate) struct Replies<K, S, H, M> {
    connector: Arc<K>,
    session: Arc<S>,
    mailbox: M,
    chat: Option<H>,
    runner: Option<Runner>,
    live: Option<Live>,
    asking: Asking,
    asked: Option<Permission>,
    next_id: u64,
}

#[derive(Debug)]
pub(crate) struct ReplyNews {
    id: u64,
    news: News,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum News {
    Piece(ReplyEvent),
    Asked(Permission),
    Ended(Result<Reply, Problem>),
}

struct Live {
    id: u64,
    cancellation: Cancellation,
}

impl<K, S, H, M> Replies<K, S, H, M>
where
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
    M: Mailbox<ReplyNews>,
{
    pub(crate) fn new(
        connector: Arc<K>,
        session: Arc<S>,
        chat: H,
        mode: PermissionMode,
        mailbox: M,
    ) -> Self {
        Self {
            connector,
            session,
            mailbox,
            chat: Some(chat),
            runner: None,
            live: None,
            asking: Asking::new(mode),
            asked: None,
            next_id: 0,
        }
    }

    pub(crate) fn ask(&mut self, prompt: Prompt) {
        let id = self.next_id;
        self.next_id += 1;
        let cancellation = Cancellation::new();
        let Some(runner) = self.runner() else {
            return;
        };
        runner.send(Job::Reply {
            id,
            prompt,
            cancellation: cancellation.clone(),
        });
        self.live = Some(Live { id, cancellation });
    }

    pub(crate) fn start_over(&mut self) {
        if let Some(runner) = self.runner() {
            runner.send(Job::Reset);
        }
    }

    pub(crate) fn stop(&mut self) -> bool {
        self.asked = None;
        let Some(live) = self.live.take() else {
            return false;
        };
        live.cancellation.cancel();
        true
    }

    pub(crate) fn hear(&mut self, news: ReplyNews) -> Option<News> {
        if self.live.as_ref()?.id != news.id {
            return None;
        }
        match &news.news {
            News::Asked(permission) => self.asked = Some(permission.clone()),
            News::Ended(_) => {
                self.live = None;
                self.asked = None;
            }
            News::Piece(_) => {}
        }
        Some(news.news)
    }

    pub(crate) fn answer(&mut self, answer: Answer) -> bool {
        self.asked
            .take()
            .is_some_and(|permission| self.asking.answer(answer, &permission))
    }

    pub(crate) fn stop_all(&mut self) {
        self.stop();
        self.chat = None;
        if let Some(mut runner) = self.runner.take() {
            runner.stop();
        }
    }

    fn runner(&mut self) -> Option<&Runner> {
        if let Some(chat) = self.chat.take() {
            let crew = Crew {
                connector: Arc::clone(&self.connector),
                session: Arc::clone(&self.session),
                asking: self.asking.clone(),
                mailbox: self.mailbox.clone(),
            };
            self.runner = Some(Runner::start(chat, crew));
        }
        self.runner.as_ref()
    }
}

impl<K, S, H, M> Drop for Replies<K, S, H, M> {
    fn drop(&mut self) {
        if let Some(live) = self.live.take() {
            live.cancellation.cancel();
        }
        if let Some(mut runner) = self.runner.take() {
            runner.stop();
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/reply/mod.rs"]
pub(crate) mod tests;
