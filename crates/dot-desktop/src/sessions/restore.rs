use std::sync::Arc;
use std::time::{Duration, Instant};

use dot_api::{
    Arrangement, Happened, KeptId, KeptSession, PermissionMode, Record, Reply, ReplyFailure,
    Restored, Session, Shelf, Tuning, changed_lines,
};

use crate::app::{Opener, Parts};
use crate::conversation::Conversation;
use crate::reply::Clock;
use crate::sessions::{Sessions, WindowSession};

pub(crate) const PAUSE_BETWEEN_LAUNCHES: Duration = Duration::from_secs(5);

pub(crate) struct Making<'a, P: Parts> {
    pub(crate) open: &'a mut Opener<P>,
    pub(crate) wiring: Wiring<'a, P>,
}

pub(crate) struct Wiring<'a, P: Parts> {
    pub(crate) connector: &'a Arc<P::Connector>,
    pub(crate) answering: (PermissionMode, Tuning),
    pub(crate) mailbox: &'a P::Mailbox,
}

enum Shown {
    Restored(usize),
    Fresh(usize),
}

impl<P: Parts> Sessions<P> {
    pub(crate) fn arrangement(&self) -> Arrangement {
        Arrangement {
            sessions: self
                .opened
                .iter()
                .filter(|opened| is_kept(opened))
                .map(kept_session)
                .collect(),
            shown: self.shown_kept(),
        }
    }

    pub(crate) fn is_arranged_as(&self, arrangement: &Arrangement) -> bool {
        let mut kept = self.opened.iter().filter(|opened| is_kept(opened));
        let same_sessions = arrangement
            .sessions
            .iter()
            .all(|session| kept.next().is_some_and(|opened| is(opened, session)));
        same_sessions && kept.next().is_none() && arrangement.shown == self.shown_kept()
    }

    pub(crate) fn kept_ids(&self) -> Vec<KeptId> {
        self.opened.iter().map(|opened| opened.kept).collect()
    }

    pub(crate) fn restore(
        &mut self,
        kept: Shelf,
        making: Making<'_, P>,
        (now, translator): (Instant, &P::Translator),
    ) {
        let Shelf { sessions, shown } = kept;
        if sessions.is_empty() {
            return;
        }
        let Making { open, wiring } = making;
        let restored: Vec<WindowSession<P>> = sessions
            .into_iter()
            .map(|kept| {
                let parts = open(kept.session.folder.clone());
                self.restored(kept, parts, &wiring, (now, translator))
            })
            .collect();
        self.place(restored, shown);
    }

    fn restored(
        &mut self,
        kept: Restored,
        parts: (P::Session, P::Chat),
        wiring: &Wiring<'_, P>,
        (now, translator): (Instant, &P::Translator),
    ) -> WindowSession<P> {
        let Restored {
            session: identity,
            records,
            memory,
        } = kept;
        let resumes_at = records.last().map_or(Duration::ZERO, |last| {
            last.at.saturating_add(PAUSE_BETWEEN_LAUNCHES)
        });
        let clock = Clock::resuming_at(resumes_at);
        let (mode, tuning) = wiring.answering;
        let answering = ((mode, kept_tuning(&identity, tuning)), wiring.mailbox);
        let mut opened = self.build(
            (identity.id, clock),
            parts,
            wiring.connector,
            answering,
            identity.hue,
        );

        spend_again(opened.session.as_ref(), &records);
        let start = now.checked_sub(resumes_at).unwrap_or(now);
        let mut conversation = Conversation::default();
        conversation.replay(records, start, translator);
        conversation.pace_replies();
        opened.conversation = conversation;
        if let Some(memory) = memory {
            opened.replies.recall(memory);
        }
        opened.identity.name = identity.name;
        opened.folder = identity.folder;
        opened
    }

    fn place(&mut self, restored: Vec<WindowSession<P>>, shown: Option<KeptId>) {
        let fresh = std::mem::take(&mut self.opened);
        let mut kept_fresh = Vec::new();
        let mut active = None;
        for (index, session) in fresh.into_iter().enumerate() {
            let twin = restored
                .iter()
                .position(|kept| session.folder.is_some() && kept.folder == session.folder);
            let replaced =
                session.conversation.is_untouched() && (session.folder.is_none() || twin.is_some());
            if index == self.active {
                active = match (replaced, twin) {
                    (true, Some(twin)) => Some(Shown::Restored(twin)),
                    (true, None) => None,
                    (false, _) => Some(Shown::Fresh(kept_fresh.len())),
                };
            }
            if !replaced {
                kept_fresh.push(session);
            }
        }

        let restored_count = restored.len();
        let shown_restored = restored.iter().position(|kept| Some(kept.kept) == shown);
        self.opened = restored;
        self.opened.extend(kept_fresh);
        self.active = match active {
            Some(Shown::Restored(index)) => index,
            Some(Shown::Fresh(index)) => restored_count + index,
            None => shown_restored.unwrap_or(restored_count - 1),
        };
        for (number, opened) in (1..).zip(self.opened.iter_mut()) {
            opened.number = number;
        }
        self.opened[self.active].unread = None;
    }

    fn shown_kept(&self) -> Option<KeptId> {
        let shown = self.active();
        is_kept(shown).then_some(shown.kept)
    }
}

fn is_kept<P: Parts>(opened: &WindowSession<P>) -> bool {
    opened.conversation.has_exchanges() || opened.identity.name.is_some()
}

fn kept_session<P: Parts>(opened: &WindowSession<P>) -> KeptSession {
    KeptSession {
        id: opened.kept,
        name: opened.identity.name.clone(),
        hue: opened.identity.hue,
        folder: opened.folder.clone(),
        model: Some(opened.tuning.model),
        effort: Some(opened.tuning.effort),
    }
}

fn kept_tuning(session: &KeptSession, tuning: Tuning) -> Tuning {
    Tuning {
        model: session.model.unwrap_or(tuning.model),
        effort: session.effort.unwrap_or(tuning.effort),
        ..tuning
    }
}

fn is<P: Parts>(opened: &WindowSession<P>, session: &KeptSession) -> bool {
    opened.kept == session.id
        && opened.identity.name == session.name
        && opened.identity.hue == session.hue
        && opened.folder == session.folder
        && Some(opened.tuning.model) == session.model
        && Some(opened.tuning.effort) == session.effort
}

fn spend_again(session: &impl Session, records: &[Record<'_>]) {
    for record in records {
        match &record.happened {
            Happened::Ended(ending) => {
                session.record(&ending.spent);
                let crashed = ending
                    .failure
                    .as_ref()
                    .is_some_and(|failure| failure.text == ReplyFailure::Crashed.text());
                if crashed {
                    session.clear_context();
                }
            }
            Happened::Heard(event) => {
                if let Some((added, removed)) = changed_lines(event) {
                    session.count_changes(added, removed);
                }
            }
            Happened::StartedOver => session.clear_context(),
            Happened::Carried(carried) => {
                session.record(&Reply {
                    cost: carried.cost,
                    ..Reply::default()
                });
                session.count_changes(carried.lines_added, carried.lines_removed);
            }
            Happened::Asked(_) => {}
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sessions/restore.rs"]
mod tests;
