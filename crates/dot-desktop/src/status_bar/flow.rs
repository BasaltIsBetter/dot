use std::sync::Arc;

use dot_api::{Cost, Limits, Reply, Session, SessionStatus, Translator, Usage};

use crate::canvas::Strip;
use crate::status_bar::arrange::arrange;
use crate::status_bar::view::modules;
use crate::worker::{Mailbox, Workers};

pub(crate) struct StatusBar<S, M> {
    session: Arc<S>,
    workers: Workers<M>,
    status: Option<SessionStatus>,
    replied: Usage,
    limits: Limits,
}

impl<S: Session + 'static, M: Mailbox<SessionStatus>> StatusBar<S, M> {
    pub(crate) fn new(session: S, mailbox: M) -> Self {
        Self {
            session: Arc::new(session),
            workers: Workers::new(mailbox),
            status: None,
            replied: Usage::default(),
            limits: Limits::default(),
        }
    }

    pub(crate) fn refresh(&mut self) {
        let session = Arc::clone(&self.session);
        self.workers.spawn(move |_| session.status());
    }

    pub(crate) fn report(&mut self, status: SessionStatus) {
        self.status = Some(status);
        self.show_replies();
    }

    pub(crate) fn record(&mut self, reply: &Reply) {
        let spent = self
            .replied
            .cost
            .micro_dollars()
            .saturating_add(reply.cost.micro_dollars());
        self.replied.cost = Cost::from_micro_dollars(spent);
        self.replied.context_tokens = reply.context_tokens;
        self.limits.five_hours = reply.limits.five_hours.or(self.limits.five_hours);
        self.limits.seven_days = reply.limits.seven_days.or(self.limits.seven_days);
        self.show_replies();
    }

    pub(crate) fn count_changes(&mut self, added: u64, removed: u64) {
        self.replied.lines_added = self.replied.lines_added.saturating_add(added);
        self.replied.lines_removed = self.replied.lines_removed.saturating_add(removed);
        self.show_replies();
    }

    pub(crate) fn strip(&self, translator: &impl Translator, columns: u16) -> Option<Strip> {
        let status = self.status.as_ref()?;
        Some(arrange(modules(status, translator), columns))
    }

    pub(crate) fn stop(&mut self) {
        self.workers.stop_all();
    }

    fn show_replies(&mut self) {
        let Some(status) = self.status.as_mut() else {
            return;
        };
        status.usage.context_tokens = self.replied.context_tokens;
        status.usage.lines_added = self.replied.lines_added;
        status.usage.lines_removed = self.replied.lines_removed;
        status.usage.cost = self.replied.cost;
        status.limits.five_hours = self.limits.five_hours.or(status.limits.five_hours);
        status.limits.seven_days = self.limits.seven_days.or(status.limits.seven_days);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/flow.rs"]
pub(crate) mod tests;
