use std::sync::Arc;

use dot_api::{Session, SessionStatus, Spending, Translator};

use crate::canvas::{Layout, Strip};
use crate::status_bar::arrange::arrange;
use crate::status_bar::view::{Activity, items};
use crate::worker::{Mailbox, Workers};

pub(crate) struct StatusBar<M> {
    workers: Workers<M>,
    status: Option<SessionStatus>,
    shown: Option<Shown>,
}

struct Shown {
    status: SessionStatus,
    activity: Activity,
    columns: u16,
    middle: bool,
    strip: Strip,
}

impl<M: Mailbox<SessionStatus>> StatusBar<M> {
    pub(crate) fn new(mailbox: M) -> Self {
        Self {
            workers: Workers::new(mailbox),
            status: None,
            shown: None,
        }
    }

    pub(crate) fn refresh<S: Session + 'static>(&mut self, session: Arc<S>) {
        self.workers.spawn(move |_| session.status(), || None);
    }

    pub(crate) fn project(&self) -> Option<&str> {
        self.status.as_ref()?.project.as_deref()
    }

    pub(crate) fn report(&mut self, status: SessionStatus) {
        self.status = Some(status);
    }

    pub(crate) fn strip(
        &mut self,
        translator: &impl Translator,
        layout: &Layout,
        activity: &Activity,
        Spending { usage, limits }: Spending,
    ) -> Option<Strip> {
        let mut status = self.status.clone()?;
        status.usage = usage;
        status.limits.five_hours = limits.five_hours.or(status.limits.five_hours);
        status.limits.seven_days = limits.seven_days.or(status.limits.seven_days);

        let columns = layout.strip_columns();
        let middle_shown = layout.has_room_for_the_middle_zone();
        let unchanged = self.shown.as_ref().is_some_and(|shown| {
            shown.columns == columns
                && shown.middle == middle_shown
                && shown.status == status
                && &shown.activity == activity
        });
        if !unchanged {
            let strip = arrange(items(&status, activity, translator), columns, middle_shown);
            self.shown = Some(Shown {
                status,
                activity: activity.clone(),
                columns,
                middle: middle_shown,
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
