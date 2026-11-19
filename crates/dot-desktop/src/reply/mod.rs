mod asking;

use std::sync::Arc;

use dot_api::{
    Cancellation, Chat, Connector, Permission, PermissionMode, Prompt, Reply, ReplyEvent,
};

use crate::error::Problem;
use crate::worker::{Mailbox, Workers};

pub(crate) use self::asking::Answer;
use self::asking::{Asker, Asking};

pub(crate) struct Replies<K, H, M> {
    connector: Arc<K>,
    chat: Arc<H>,
    mode: PermissionMode,
    workers: Workers<M>,
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

impl<K, H, M> Replies<K, H, M>
where
    K: Connector + 'static,
    H: Chat + 'static,
    M: Mailbox<ReplyNews>,
{
    pub(crate) fn new(connector: Arc<K>, chat: H, mode: PermissionMode, mailbox: M) -> Self {
        Self {
            connector,
            chat: Arc::new(chat),
            mode,
            workers: Workers::new(mailbox),
            live: None,
            asking: Asking::default(),
            asked: None,
            next_id: 0,
        }
    }

    pub(crate) fn ask(&mut self, prompt: Prompt) {
        let id = self.next_id;
        self.next_id += 1;
        let connector = Arc::clone(&self.connector);
        let chat = Arc::clone(&self.chat);
        let mode = self.mode;
        let asking = self.asking.clone();

        let cancellation = self.workers.spawn_streaming(move |cancellation, deliver| {
            let asker = Asker {
                id,
                mode,
                asking,
                deliver,
            };
            let ended = connector
                .credential()
                .map_err(|error| Problem::new(error.failure().text(), &error))
                .and_then(|credential| {
                    chat.reply(&prompt, &credential, cancellation, &asker, &mut |event| {
                        deliver(ReplyNews {
                            id,
                            news: News::Piece(event),
                        });
                    })
                    .map_err(|error| Problem::new(error.failure().text(), &error))
                });
            ReplyNews {
                id,
                news: News::Ended(ended),
            }
        });
        self.live = Some(Live { id, cancellation });
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
        self.live = None;
        self.asked = None;
        self.workers.stop_all();
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/reply/mod.rs"]
pub(crate) mod tests;
