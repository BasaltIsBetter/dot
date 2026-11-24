use std::sync::Arc;
use std::time::{Duration, Instant};

use dot_api::{Session, SessionStatus, Spending, Translator, Tuning};

use crate::canvas::{Layout, Strip};
use crate::sessions::SessionKey;
use crate::status_bar::arrange::arrange;
use crate::status_bar::view::{Activity, Searching, items, searching_items};
use crate::worker::{Mailbox, Workers};
use dot_transcript::{Glide, Ink, Motion, Palette, SETTLE, opacity};

const FULL_FROM_PER_MILLE: u64 = 800;
const PER_MILLE: u64 = 1_000;
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));

pub(crate) struct StatusBar<M> {
    workers: Workers<M>,
    status: Option<SessionStatus>,
    shown: Option<Shown>,
    gauge_of: Option<SessionKey>,
    context_seen: Option<u64>,
    grew_at: Option<Instant>,
    fill: Option<Glide>,
}

struct Shown {
    status: SessionStatus,
    activity: Activity,
    tuning: Tuning,
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
            gauge_of: None,
            context_seen: None,
            grew_at: None,
            fill: None,
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
        self.context_seen = None;
        self.grew_at = None;
        self.fill = None;
    }

    pub(crate) fn gauge_for(&mut self, session: SessionKey) {
        if self.gauge_of.replace(session) != Some(session) {
            self.context_seen = None;
            self.grew_at = None;
        }
    }

    pub(crate) fn gauge_fill(&mut self, (used, window): (u64, u64), now: Instant) -> u16 {
        let share = used.saturating_mul(PER_MILLE) / window.max(1);
        let target = f32::from(u16::try_from(share.min(PER_MILLE)).unwrap_or(u16::MAX));
        let fill = self
            .fill
            .get_or_insert_with(|| Glide::at_rest(target, SETTLE));
        fill.toward(target, now);
        whole(fill.position(now))
    }

    pub(crate) fn gauge_tint(
        &mut self,
        (used, window): (u64, u64),
        colour: Option<[u8; 3]>,
        now: Instant,
    ) -> Option<[u8; 3]> {
        let share = used.saturating_mul(PER_MILLE) / window.max(1);
        if self.context_seen.is_some_and(|seen| share > seen) {
            self.grew_at = Some(now);
        }
        self.context_seen = Some(share);
        let colour = colour?;
        if share >= FULL_FROM_PER_MILLE {
            return Some(colour);
        }
        let eased = self
            .grew_at
            .and_then(|at| COOLING.eased(now.saturating_duration_since(at)))?;
        Some(Palette { live: colour }.shade(Ink::Full, opacity(1.0 - eased), u8::MAX))
    }

    pub(crate) fn is_cooling(&self, now: Instant) -> bool {
        let gliding = self.fill.is_some_and(|fill| fill.is_moving(now));
        gliding
            || self
                .grew_at
                .is_some_and(|at| COOLING.lasts(now.saturating_duration_since(at)))
    }

    pub(crate) fn strip(
        &mut self,
        translator: &impl Translator,
        layout: &Layout,
        (activity, tuning): (&Activity, &Tuning),
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
                && &shown.tuning == tuning
        });
        if !unchanged {
            let items = items(&status, (activity, tuning), translator);
            let strip = arrange(items, columns, middle_shown);
            self.shown = Some(Shown {
                status,
                activity: activity.clone(),
                tuning: *tuning,
                columns,
                middle: middle_shown,
                strip,
            });
        }
        self.shown.as_ref().map(|shown| shown.strip.clone())
    }

    pub(crate) fn searching(
        translator: &impl Translator,
        layout: &Layout,
        searching: &Searching,
    ) -> Strip {
        let columns = layout.strip_columns();
        let middle_shown = layout.has_room_for_the_middle_zone();
        arrange(
            searching_items(searching, translator),
            columns,
            middle_shown,
        )
    }

    pub(crate) fn stop(&mut self) {
        self.workers.stop_all();
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/flow.rs"]
pub(crate) mod tests;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a share in thousandths lies between 0 and 1,000"
)]
fn whole(per_mille: f32) -> u16 {
    per_mille.round().clamp(0.0, 1_000.0) as u16
}
