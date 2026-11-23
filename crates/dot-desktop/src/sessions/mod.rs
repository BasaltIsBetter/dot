mod card;
mod key;
mod rail;

use std::sync::Arc;
use std::time::{Duration, Instant};

use dot_api::{PermissionMode, Translator};

use crate::app::Parts;
use crate::conversation::Conversation;
use crate::reply::Replies;
use dot_transcript::{Motion, opacity};

use self::key::Tagged;

pub(crate) use self::card::Card;
#[cfg(test)]
pub(crate) use self::card::{Foot, State};
pub(crate) use self::key::SessionKey;
pub(crate) use self::rail::{Hit, Rail};

const SWITCHING: Motion = Motion::landing(Duration::from_millis(450));
const BLINK_STEP: Duration = Duration::from_millis(50);
const UNREAD_MARK: &str = "◆";
const SMALLEST_SCALE: f64 = 0.97;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Switching {
    pub(crate) alpha: u8,
    pub(crate) scale: f64,
}

pub(crate) struct WindowSession<P: Parts> {
    pub(crate) key: SessionKey,
    pub(crate) number: u64,
    pub(crate) session: Arc<P::Session>,
    pub(crate) replies: Replies<P::Connector, P::Session, P::Chat, Tagged<P::Mailbox>>,
    pub(crate) conversation: Conversation,
    pub(crate) unread: Option<Instant>,
}

pub(crate) struct Sessions<P: Parts> {
    opened: Vec<WindowSession<P>>,
    active: usize,
    created: u64,
    switched: Option<Instant>,
}

impl<P: Parts> Sessions<P> {
    pub(crate) fn new(
        parts: (P::Session, P::Chat),
        connector: &Arc<P::Connector>,
        mode: PermissionMode,
        mailbox: &P::Mailbox,
    ) -> Self {
        let mut sessions = Self {
            opened: Vec::new(),
            active: 0,
            created: 0,
            switched: None,
        };
        sessions.push(parts, connector, mode, mailbox);
        sessions
    }

    pub(crate) fn open(
        &mut self,
        parts: (P::Session, P::Chat),
        connector: &Arc<P::Connector>,
        (mode, mailbox): (PermissionMode, &P::Mailbox),
        now: Instant,
    ) {
        self.push(parts, connector, mode, mailbox);
        self.switch_to(self.opened.len() - 1, now);
    }

    pub(crate) fn active(&self) -> &WindowSession<P> {
        &self.opened[self.active]
    }

    pub(crate) fn active_mut(&mut self) -> &mut WindowSession<P> {
        &mut self.opened[self.active]
    }

    pub(crate) fn switch_to(&mut self, index: usize, now: Instant) {
        if index == self.active || index >= self.opened.len() {
            return;
        }
        self.active = index;
        self.opened[index].unread = None;
        self.switched = Some(now);
    }

    pub(crate) fn step(&mut self, step: Step, now: Instant) {
        let count = self.opened.len();
        let index = match step {
            Step::Previous => (self.active + count - 1) % count,
            Step::Next => (self.active + 1) % count,
        };
        self.switch_to(index, now);
    }

    pub(crate) fn close(&mut self, index: usize, now: Instant) {
        if self.opened.len() < 2 || index >= self.opened.len() {
            return;
        }
        let mut closing = self.opened.remove(index);
        closing.replies.stop_all();
        if index == self.active {
            self.active = index.min(self.opened.len() - 1);
            self.opened[self.active].unread = None;
            self.switched = Some(now);
        } else if index < self.active {
            self.active -= 1;
        }
    }

    pub(crate) fn by_key_mut(&mut self, key: SessionKey) -> Option<(bool, &mut WindowSession<P>)> {
        let active = self.active;
        self.opened
            .iter_mut()
            .enumerate()
            .find(|(_, opened)| opened.key == key)
            .map(|(index, opened)| (index == active, opened))
    }

    pub(crate) fn unread(&self) -> usize {
        self.opened
            .iter()
            .filter(|opened| opened.unread.is_some())
            .count()
    }

    pub(crate) fn next_blink(&self, now: Instant) -> Option<Instant> {
        (self.unread() > 0).then(|| now + BLINK_STEP)
    }

    pub(crate) fn count(&self) -> String {
        let shown = format!("{}/{}", self.active + 1, self.opened.len());
        match self.unread() {
            0 => shown,
            unread => format!("{shown} {UNREAD_MARK}{unread}"),
        }
    }

    pub(crate) fn switching(&self, now: Instant) -> Option<Switching> {
        let switched = self.switched?;
        let eased = SWITCHING.eased(now.saturating_duration_since(switched))?;
        Some(Switching {
            alpha: opacity(eased),
            scale: SMALLEST_SCALE + (1.0 - SMALLEST_SCALE) * eased,
        })
    }

    pub(crate) fn cards(
        &self,
        project: Option<&str>,
        now: Instant,
        translator: &impl Translator,
    ) -> Vec<Card> {
        self.opened
            .iter()
            .enumerate()
            .map(|(index, opened)| Card::of(opened, index == self.active, project, now, translator))
            .collect()
    }

    pub(crate) fn stop_all(&mut self) {
        for opened in &mut self.opened {
            opened.replies.stop_all();
        }
    }

    fn push(
        &mut self,
        (session, chat): (P::Session, P::Chat),
        connector: &Arc<P::Connector>,
        mode: PermissionMode,
        mailbox: &P::Mailbox,
    ) {
        self.created += 1;
        let key = SessionKey(self.created);
        let session = Arc::new(session);
        let tagged = Tagged::new(mailbox.clone(), key);
        self.opened.push(WindowSession {
            key,
            number: self.created,
            replies: Replies::new(
                Arc::clone(connector),
                Arc::clone(&session),
                chat,
                mode,
                tagged,
            ),
            session,
            conversation: Conversation::default(),
            unread: None,
        });
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sessions/mod.rs"]
pub(crate) mod tests;
