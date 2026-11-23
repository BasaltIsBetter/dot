use std::time::{Duration, Instant};

use dot_api::{ReplyEvent, Text};

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::{Intent, intent};
use crate::sessions::tests::sessions;
use winit::keyboard::{Key, ModifiersState, NamedKey};

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn send(conversation: &mut crate::conversation::Conversation, text: &str, now: Instant) {
    let insert = Intent::Edit(crate::conversation::Edit::Insert(text.to_owned()));
    conversation.type_key(insert, now, || None, &IdTranslator);
    let enter = intent(&Key::Named(NamedKey::Enter), None, ModifiersState::empty()).unwrap();
    conversation.type_key(enter, now, || None, &IdTranslator);
}

#[test]
fn names_the_first_session_after_the_project_and_the_others_by_number() {
    let now = Instant::now();
    let (sessions, _receiver) = sessions(2, now);

    let cards = sessions.cards(Some("Dot"), now, &IdTranslator);
    let unnamed = sessions.cards(None, now, &IdTranslator);

    assert_eq!(cards[0].name, "Dot");
    assert_eq!(cards[1].name, format!("{} 2", Text::SessionName.id()));
    assert_eq!(unnamed[0].name, format!("{} 1", Text::SessionName.id()));
}

#[test]
fn says_which_session_is_shown_writing_paused_unread_or_idle() {
    let now = Instant::now();
    let (mut sessions, _receiver) = sessions(4, now);
    send(&mut sessions.opened[1].conversation, "hello", now);
    send(&mut sessions.opened[2].conversation, "hello", now);
    sessions.opened[2]
        .conversation
        .type_key(Intent::Pause, now, || None, &IdTranslator);
    sessions.opened[3].unread = Some(now);
    sessions.switch_to(0, now);

    let states: Vec<(bool, State)> = sessions
        .cards(None, now, &IdTranslator)
        .into_iter()
        .map(|card| (card.here, card.state))
        .collect();

    assert_eq!(
        states,
        [
            (true, State::Idle),
            (false, State::Writing),
            (false, State::Paused),
            (false, State::Unread(now)),
        ]
    );
}

#[test]
fn carries_the_rate_and_the_spark_of_the_last_reply() {
    let now = Instant::now();
    let (mut sessions, _receiver) = sessions(1, now);
    let before = sessions.cards(None, now, &IdTranslator);
    let conversation = &mut sessions.opened[0].conversation;
    send(conversation, "hello", now);
    conversation.hear(
        ReplyEvent::Text("a".repeat(230)),
        after(now, 100),
        &IdTranslator,
    );

    let writing = sessions.cards(None, after(now, 500), &IdTranslator);

    assert_eq!(before[0].foot, None);
    let foot = writing[0].foot.unwrap();
    assert_eq!(foot.rate, Some(100));
    assert!(foot.rates.iter().any(|&rate| rate > 0));
}
