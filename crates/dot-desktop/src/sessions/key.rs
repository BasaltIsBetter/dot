use crate::reply::ReplyNews;
use crate::worker::Mailbox;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SessionKey(pub(crate) u64);

#[derive(Clone)]
pub(crate) struct Tagged<M> {
    mailbox: M,
    key: SessionKey,
}

impl<M> Tagged<M> {
    pub(super) const fn new(mailbox: M, key: SessionKey) -> Self {
        Self { mailbox, key }
    }
}

impl<M: Mailbox<(SessionKey, ReplyNews)>> Mailbox<ReplyNews> for Tagged<M> {
    fn deliver(&self, news: ReplyNews) {
        self.mailbox.deliver((self.key, news));
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sessions/key.rs"]
mod tests;
