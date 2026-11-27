use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use dot_api::{Hue, PermissionMode};

use crate::settings::tests::test_settings;

use super::*;
use crate::app::tests::TestParts;
use crate::event::Delivery;
use crate::keeping::tests::MemoryArchive;
use crate::reply::Opened;
use crate::reply::tests::{ScriptedChat, asked};
use crate::sign_in::FakeConnector;
use crate::status_bar::FixedSession;

pub(crate) type TestSessions = Sessions<TestParts>;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

pub(crate) fn sessions(count: usize) -> (TestSessions, Receiver<Delivery>) {
    let (sender, receiver) = mpsc::channel();
    let connector = Arc::new(FakeConnector::default());
    let parts = || (FixedSession::default(), ScriptedChat::default());
    let mut sessions = Sessions::new(
        parts(),
        &connector,
        ((PermissionMode::Ask, test_settings().tuning), &sender),
        (Hue::Violet, 7),
        Arc::new(MemoryArchive::default()),
    );
    for _ in 1..count {
        sessions.open(
            parts(),
            &connector,
            ((PermissionMode::Ask, test_settings().tuning), &sender),
        );
    }
    (sessions, receiver)
}

#[test]
fn opens_with_one_session_shown() {
    let (sessions, _receiver) = sessions(1);

    assert_eq!(sessions.opened.len(), 1);
    assert_eq!(sessions.active().number, 1);
}

#[test]
fn shows_each_new_session_with_the_next_number() {
    let (sessions, _receiver) = sessions(3);

    assert_eq!(sessions.active, 2);
    assert_eq!(sessions.active().number, 3);
    assert_ne!(sessions.opened[0].key, sessions.opened[2].key);
}

#[test]
fn stops_the_replies_of_every_session() {
    let (mut sessions, _receiver) = sessions(2);
    for opened in &mut sessions.opened {
        opened.replies.ask(asked("Salut"), Opened::default());
    }

    sessions.stop_all();

    assert!(
        sessions
            .opened
            .iter_mut()
            .all(|opened| !opened.replies.stop())
    );
}

#[test]
fn steps_to_the_session_above_or_below_round_the_list() {
    let (mut sessions, _receiver) = sessions(3);

    sessions.step(Step::Next);
    let wrapped = sessions.active;
    sessions.step(Step::Previous);

    assert_eq!(wrapped, 0);
    assert_eq!(sessions.active, 2);
}

#[test]
fn switching_to_the_session_shown_or_to_none_changes_nothing() {
    let (mut sessions, _receiver) = sessions(2);

    sessions.switch_to(1);
    sessions.switch_to(9);

    assert_eq!(sessions.active, 1);
}

#[test]
fn opening_a_session_marks_it_read() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2);
    sessions.opened[0].unread = Some(start);

    let unread = sessions.unread();
    sessions.switch_to(0);

    assert_eq!(unread, 1);
    assert_eq!(sessions.unread(), 0);
    assert_eq!(sessions.opened[0].unread, None);
}

#[test]
fn closing_the_session_shown_shows_its_neighbour() {
    let (mut sessions, _receiver) = sessions(3);
    sessions.switch_to(1);
    let third = sessions.opened[2].key;

    sessions.close(1);

    assert_eq!(sessions.opened.len(), 2);
    assert_eq!(sessions.active().key, third);

    sessions.close(1);

    assert_eq!(sessions.active, 0);
}

#[test]
fn closing_another_session_keeps_the_one_shown() {
    let (mut sessions, _receiver) = sessions(3);
    let shown = sessions.active().key;

    sessions.close(0);

    assert_eq!(sessions.active().key, shown);
    assert_eq!(sessions.active, 1);
}

#[test]
fn never_closes_the_last_session() {
    let (mut sessions, _receiver) = sessions(1);

    sessions.close(0);
    sessions.close(5);

    assert_eq!(sessions.opened.len(), 1);
}

#[test]
fn finds_a_session_by_its_key_and_says_whether_it_is_shown() {
    let (mut sessions, _receiver) = sessions(2);
    let first = sessions.opened[0].key;
    let second = sessions.opened[1].key;

    assert_eq!(
        sessions.by_key_mut(first).map(|(shown, _)| shown),
        Some(false)
    );
    assert_eq!(
        sessions.by_key_mut(second).map(|(shown, _)| shown),
        Some(true)
    );
    assert!(sessions.by_key_mut(SessionKey(99)).is_none());
}

#[test]
fn counts_the_sessions_and_those_unread_for_a_narrow_window() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(4);
    sessions.switch_to(0);

    let read = sessions.count();
    sessions.opened[2].unread = Some(start);

    assert_eq!(read, "1/4");
    assert_eq!(sessions.count(), "1/4 ◆1");
}

#[test]
fn wakes_every_fiftieth_of_a_second_while_a_session_is_unread() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2);

    let read = sessions.next_blink(start);
    sessions.opened[0].unread = Some(start);

    assert_eq!(read, None);
    assert_eq!(sessions.next_blink(start), Some(after(start, 50)));
}

#[test]
fn tunes_every_session_and_switches_all_their_permissions() {
    let (sender, _receiver) = mpsc::channel();
    let connector = Arc::new(FakeConnector::default());
    let first = ScriptedChat::default();
    let second = ScriptedChat::default();
    let tunings = [Arc::clone(&first.tunings), Arc::clone(&second.tunings)];
    let answering = (PermissionMode::Ask, test_settings().tuning);
    let mut sessions: TestSessions = Sessions::new(
        (FixedSession::default(), first),
        &connector,
        (answering, &sender),
        (Hue::Violet, 7),
        Arc::new(MemoryArchive::default()),
    );
    sessions.open(
        (FixedSession::default(), second),
        &connector,
        (answering, &sender),
    );
    let haiku = dot_api::Tuning {
        model: dot_api::Model::HAIKU_5_5,
        ..test_settings().tuning
    };

    sessions.tune(haiku);
    sessions.permit(PermissionMode::Bypass);

    for tunings in &tunings {
        assert_eq!(*tunings.lock().unwrap(), [test_settings().tuning, haiku]);
    }
}

#[test]
fn gives_each_new_session_a_colour_the_others_do_not_have() {
    let (sessions, _receiver) = sessions(8);

    let mut hues: Vec<Hue> = (0..8)
        .map(|index| sessions.opened[index].identity.hue)
        .collect();
    assert_eq!(hues[0], Hue::Violet);
    hues.sort_by_key(|hue| Hue::ALL.iter().position(|all| all == hue));
    assert_eq!(hues, Hue::ALL);
}

#[test]
fn paints_only_the_session_shown() {
    let (mut sessions, _receiver) = sessions(2);
    let first = sessions.opened[0].identity.hue;

    sessions.paint(Hue::Amber);

    assert_eq!(sessions.hue(), Hue::Amber);
    assert_eq!(sessions.opened[1].identity.hue, Hue::Amber);
    assert_eq!(sessions.opened[0].identity.hue, first);
    sessions.switch_to(0);
    assert_eq!(sessions.hue(), first);
}
