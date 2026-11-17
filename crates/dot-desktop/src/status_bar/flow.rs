use std::sync::Arc;

use dot_api::{Session, SessionStatus, Translator};

use crate::canvas::Strip;
use crate::status_bar::arrange::arrange;
use crate::status_bar::view::modules;
use crate::worker::{Mailbox, Workers};

pub(crate) struct StatusBar<S, M> {
    session: Arc<S>,
    workers: Workers<M>,
    status: Option<SessionStatus>,
}

impl<S: Session + 'static, M: Mailbox<SessionStatus>> StatusBar<S, M> {
    pub(crate) fn new(session: S, mailbox: M) -> Self {
        Self {
            session: Arc::new(session),
            workers: Workers::new(mailbox),
            status: None,
        }
    }

    pub(crate) fn refresh(&mut self) {
        let session = Arc::clone(&self.session);
        self.workers.spawn(move |_| session.status());
    }

    pub(crate) fn report(&mut self, status: SessionStatus) {
        self.status = Some(status);
    }

    pub(crate) fn strip(&self, translator: &impl Translator, columns: u16) -> Option<Strip> {
        let status = self.status.as_ref()?;
        Some(arrange(modules(status, translator), columns))
    }

    pub(crate) fn stop(&mut self) {
        self.workers.stop_all();
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/flow.rs"]
pub(crate) mod tests;
