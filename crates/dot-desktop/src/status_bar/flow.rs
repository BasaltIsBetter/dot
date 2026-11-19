use std::sync::Arc;

use dot_api::{Session, SessionStatus, Spending, Translator};

use crate::canvas::Strip;
use crate::status_bar::arrange::arrange;
use crate::status_bar::view::modules;
use crate::worker::{Mailbox, Workers};

pub(crate) struct StatusBar<S, M> {
    session: Arc<S>,
    workers: Workers<M>,
    status: Option<SessionStatus>,
    shown: Option<Shown>,
}

struct Shown {
    status: SessionStatus,
    columns: u16,
    strip: Strip,
}

impl<S: Session + 'static, M: Mailbox<SessionStatus>> StatusBar<S, M> {
    pub(crate) fn new(session: Arc<S>, mailbox: M) -> Self {
        Self {
            session,
            workers: Workers::new(mailbox),
            status: None,
            shown: None,
        }
    }

    pub(crate) fn refresh(&mut self) {
        let session = Arc::clone(&self.session);
        self.workers.spawn(move |_| session.status(), || None);
    }

    pub(crate) fn report(&mut self, status: SessionStatus) {
        self.status = Some(status);
    }

    pub(crate) fn strip(&mut self, translator: &impl Translator, columns: u16) -> Option<Strip> {
        let mut status = self.status.clone()?;
        let Spending { usage, limits } = self.session.spending();
        status.usage = usage;
        status.limits.five_hours = limits.five_hours.or(status.limits.five_hours);
        status.limits.seven_days = limits.seven_days.or(status.limits.seven_days);

        let unchanged = self
            .shown
            .as_ref()
            .is_some_and(|shown| shown.columns == columns && shown.status == status);
        if !unchanged {
            let strip = arrange(modules(&status, translator), columns);
            self.shown = Some(Shown {
                status,
                columns,
                strip,
            });
        }
        self.shown.as_ref().map(|shown| shown.strip.clone())
    }

    pub(crate) fn stop(&mut self) {
        self.workers.stop_all();
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/flow.rs"]
pub(crate) mod tests;
