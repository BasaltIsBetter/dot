use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use dot_api::PermissionMode;

use super::*;
use crate::app::tests::TestParts;
use crate::event::Delivery;
use crate::reply::tests::{ScriptedChat, asked};
use crate::sign_in::FakeConnector;
use crate::status_bar::FixedSession;

pub(crate) type TestSessions = Sessions<TestParts>;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

pub(crate) fn sessions(count: usize, now: Instant) -> (TestSessions, Receiver<Delivery>) {
    let (sender, receiver) = mpsc::channel();
    let connector = Arc::new(FakeConnector::default());
    let parts = || (FixedSession::default(), ScriptedChat::default());
    let mut sessions = Sessions::new(parts(), &connector, PermissionMode::Ask, &sender);
    for _ in 1..count {
        sessions.open(parts(), &connector, (PermissionMode::Ask, &sender), now);
    }
    (sessions, receiver)
}

#[test]
fn opens_with_one_session_shown() {
    let (sessions, _receiver) = sessions(1, Instant::now());

    assert_eq!(sessions.opened.len(), 1);
    assert_eq!(sessions.active().number, 1);
    assert_eq!(sessions.switching(Instant::now()), None);
}

#[test]
fn shows_each_new_session_with_the_next_number() {
    let start = Instant::now();

    let (sessions, _receiver) = sessions(3, start);

    assert_eq!(sessions.active, 2);
    assert_eq!(sessions.active().number, 3);
    assert_ne!(sessions.opened[0].key, sessions.opened[2].key);
}

#[test]
fn fades_a_session_in_over_four_hundred_and_fifty_milliseconds() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2, start);

    sessions.switch_to(0, after(start, 1_000));

    let starting = sessions.switching(after(start, 1_000)).unwrap();
    let halfway = sessions.switching(after(start, 1_225)).unwrap();
    assert_eq!(starting.alpha, 0);
    assert!((starting.scale - 0.97).abs() < 1e-9);
    assert!(halfway.alpha > 128 && halfway.scale > 0.99 && halfway.scale < 1.0);
    assert_eq!(sessions.switching(after(start, 1_450)), None);
}

#[test]
fn stops_the_replies_of_every_session() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2, start);
    for opened in &mut sessions.opened {
        opened.replies.ask(asked("Salut"));
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
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(3, start);

    sessions.step(Step::Next, start);
    let wrapped = sessions.active;
    sessions.step(Step::Previous, start);

    assert_eq!(wrapped, 0);
    assert_eq!(sessions.active, 2);
}

#[test]
fn switching_to_the_session_shown_or_to_none_changes_nothing() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2, start);
    let switched = sessions.switched;

    sessions.switch_to(1, after(start, 5_000));
    sessions.switch_to(9, after(start, 5_000));

    assert_eq!(sessions.active, 1);
    assert_eq!(sessions.switched, switched);
}

#[test]
fn opening_a_session_marks_it_read() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2, start);
    sessions.opened[0].unread = Some(start);

    let unread = sessions.unread();
    sessions.switch_to(0, start);

    assert_eq!(unread, 1);
    assert_eq!(sessions.unread(), 0);
    assert_eq!(sessions.opened[0].unread, None);
}

#[test]
fn closing_the_session_shown_shows_its_neighbour() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(3, start);
    sessions.switch_to(1, start);
    let third = sessions.opened[2].key;

    sessions.close(1, after(start, 10));

    assert_eq!(sessions.opened.len(), 2);
    assert_eq!(sessions.active().key, third);

    sessions.close(1, after(start, 20));

    assert_eq!(sessions.active, 0);
}

#[test]
fn closing_another_session_keeps_the_one_shown() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(3, start);
    let shown = sessions.active().key;

    sessions.close(0, start);

    assert_eq!(sessions.active().key, shown);
    assert_eq!(sessions.active, 1);
}

#[test]
fn never_closes_the_last_session() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(1, start);

    sessions.close(0, start);
    sessions.close(5, start);

    assert_eq!(sessions.opened.len(), 1);
}

#[test]
fn finds_a_session_by_its_key_and_says_whether_it_is_shown() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2, start);
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
    let (mut sessions, _receiver) = sessions(4, start);
    sessions.switch_to(0, start);

    let read = sessions.count();
    sessions.opened[2].unread = Some(start);

    assert_eq!(read, "1/4");
    assert_eq!(sessions.count(), "1/4 ◆1");
}

#[test]
fn wakes_every_fiftieth_of_a_second_while_a_session_is_unread() {
    let start = Instant::now();
    let (mut sessions, _receiver) = sessions(2, start);

    let read = sessions.next_blink(start);
    sessions.opened[0].unread = Some(start);

    assert_eq!(read, None);
    assert_eq!(sessions.next_blink(start), Some(after(start, 50)));
}
