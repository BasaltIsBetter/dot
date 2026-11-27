mod card;
mod identity;
mod key;
mod rail;

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use dot_api::{Hue, PermissionMode, Translator, Tuning};

use crate::app::Parts;
use crate::conversation::Conversation;
use crate::reply::Replies;
use crate::worker::join;

use self::identity::{Dice, Identity, fresh_hue};
use self::key::Tagged;

pub(crate) use self::card::{Card, CardLook, shown_name};
#[cfg(test)]
pub(crate) use self::card::{Foot, State};
pub(crate) use self::identity::NameField;
pub(crate) use self::key::SessionKey;
pub(crate) use self::rail::{Hit, Rail, RailScroll};

const BLINK_STEP: Duration = Duration::from_millis(50);
const UNREAD_MARK: &str = "◆";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Previous,
    Next,
}

pub(crate) struct WindowSession<P: Parts> {
    pub(crate) key: SessionKey,
    pub(crate) number: u64,
    pub(crate) session: Arc<P::Session>,
    pub(crate) replies: Replies<P::Connector, P::Session, P::Chat, Tagged<P::Mailbox>>,
    pub(crate) conversation: Conversation,
    pub(crate) unread: Option<Instant>,
    pub(crate) identity: Identity,
    pub(crate) folder: Option<PathBuf>,
}

pub(crate) struct Sessions<P: Parts> {
    opened: Vec<WindowSession<P>>,
    stopping: Vec<JoinHandle<()>>,
    active: usize,
    created: u64,
    rail: RailScroll,
    dice: Dice,
}

impl<P: Parts> Sessions<P> {
    pub(crate) fn new(
        parts: (P::Session, P::Chat),
        connector: &Arc<P::Connector>,
        (answering, mailbox): ((PermissionMode, Tuning), &P::Mailbox),
        (first, seed): (Hue, u64),
    ) -> Self {
        let mut sessions = Self {
            opened: Vec::new(),
            stopping: Vec::new(),
            active: 0,
            created: 0,
            rail: RailScroll::default(),
            dice: Dice::new(seed),
        };
        sessions.push(parts, connector, (answering, mailbox), first);
        sessions
    }

    pub(crate) fn open(
        &mut self,
        parts: (P::Session, P::Chat),
        connector: &Arc<P::Connector>,
        (answering, mailbox): ((PermissionMode, Tuning), &P::Mailbox),
    ) {
        let taken: Vec<Hue> = self
            .opened
            .iter()
            .map(|opened| opened.identity.hue)
            .collect();
        let hue = fresh_hue(&taken, self.dice.roll());
        self.push(parts, connector, (answering, mailbox), hue);
        self.switch_to(self.opened.len() - 1);
    }

    pub(crate) fn hue(&self) -> Hue {
        self.active().identity.hue
    }

    pub(crate) fn paint(&mut self, hue: Hue) {
        self.active_mut().identity.hue = hue;
    }

    pub(crate) fn active(&self) -> &WindowSession<P> {
        &self.opened[self.active]
    }

    pub(crate) fn active_mut(&mut self) -> &mut WindowSession<P> {
        &mut self.opened[self.active]
    }

    pub(crate) fn switch_to(&mut self, index: usize) {
        if index == self.active || index >= self.opened.len() {
            return;
        }
        self.opened[self.active].conversation.leave();
        self.active = index;
        self.opened[index].unread = None;
    }

    pub(crate) fn step(&mut self, step: Step) {
        let count = self.opened.len();
        let index = match step {
            Step::Previous => (self.active + count - 1) % count,
            Step::Next => (self.active + 1) % count,
        };
        self.switch_to(index);
    }

    pub(crate) fn close(&mut self, index: usize) {
        if self.opened.len() < 2 || index >= self.opened.len() {
            return;
        }
        let mut closing = self.opened.remove(index);
        self.join_stopped();
        self.stopping.extend(closing.replies.stop_all());
        if index == self.active {
            self.active = index.min(self.opened.len() - 1);
            self.opened[self.active].unread = None;
        } else if index < self.active {
            self.active -= 1;
        }
    }

    pub(crate) fn tune(&mut self, tuning: Tuning) {
        for opened in &mut self.opened {
            opened.replies.tune(tuning);
        }
    }

    pub(crate) fn permit(&self, mode: PermissionMode) {
        for opened in &self.opened {
            opened.replies.switch_to(mode);
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

    pub(crate) fn each_mut(&mut self) -> impl Iterator<Item = (bool, &mut WindowSession<P>)> {
        let active = self.active;
        self.opened
            .iter_mut()
            .enumerate()
            .map(move |(index, opened)| (index == active, opened))
    }

    pub(crate) fn are_pacing(&self) -> bool {
        self.opened
            .iter()
            .any(|opened| opened.conversation.is_pacing())
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

    pub(crate) fn cards(
        &self,
        look: &CardLook<'_>,
        now: Instant,
        translator: &impl Translator,
    ) -> Vec<Card> {
        self.opened
            .iter()
            .enumerate()
            .map(|(index, opened)| Card::of(opened, index == self.active, look, now, translator))
            .collect()
    }

    pub(crate) fn rename(&mut self, field: &NameField, shown: &str, now: Instant) {
        if let Some(opened) = self
            .opened
            .iter_mut()
            .find(|opened| opened.key == field.key)
        {
            opened.identity.rename(&field.text, shown, now);
        }
    }

    pub(crate) fn key_of(&self, index: usize) -> Option<SessionKey> {
        self.opened.get(index).map(|opened| opened.key)
    }

    pub(crate) fn by_key(&self, key: SessionKey) -> Option<&WindowSession<P>> {
        self.opened.iter().find(|opened| opened.key == key)
    }

    pub(crate) fn names_are_cooling(&self, now: Instant) -> bool {
        self.opened
            .iter()
            .any(|opened| opened.identity.is_cooling(now))
    }

    pub(crate) fn stop_all(&mut self) {
        self.stopping.extend(
            self.opened
                .iter_mut()
                .filter_map(|opened| opened.replies.stop_all()),
        );
        for thread in self.stopping.drain(..) {
            join(thread);
        }
    }

    fn join_stopped(&mut self) {
        for thread in self.stopping.extract_if(.., |thread| thread.is_finished()) {
            join(thread);
        }
    }

    fn push(
        &mut self,
        (session, chat): (P::Session, P::Chat),
        connector: &Arc<P::Connector>,
        ((mode, tuning), mailbox): ((PermissionMode, Tuning), &P::Mailbox),
        hue: Hue,
    ) {
        self.created += 1;
        let key = SessionKey(self.created);
        let session = Arc::new(session);
        let tagged = Tagged::new(mailbox.clone(), key);
        let mut replies = Replies::new(
            Arc::clone(connector),
            Arc::clone(&session),
            chat,
            mode,
            tagged,
        );
        replies.tune(tuning);
        self.opened.push(WindowSession {
            key,
            number: self.created,
            replies,
            session,
            conversation: paced(),
            unread: None,
            identity: Identity::new(hue),
            folder: None,
        });
    }
}

fn paced() -> Conversation {
    let mut conversation = Conversation::default();
    conversation.pace_replies();
    conversation
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sessions/mod.rs"]
pub(crate) mod tests;
