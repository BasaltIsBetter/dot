use std::time::Duration;

use std::sync::Arc;
use std::sync::mpsc::Receiver;

use dot_api::{Action, Decision, Detail, Permission, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::keyboard::NamedKey;

use super::*;
use crate::app::tests::{
    TestApp, act, chatting, deliver, flow_texts, named, settled, shown_strip, typed, until_replied,
};
use crate::event::Delivery;
use crate::reply::tests::ScriptedChat;

const PATIENCE: Duration = Duration::from_secs(5);

#[test]
fn claude_s_reply_streams_in_then_counts_in_the_status_bar() {
    let (mut app, receiver) = chatting(ScriptedChat {
        pieces: vec![
            ReplyEvent::Thinking("Hm.".into()),
            ReplyEvent::Text("Bonjour".into()),
        ],
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));

    until_replied(&mut app, &receiver);
    settled(&mut app);

    assert_eq!(
        flow_texts(&app),
        vec![
            "› hello",
            "Hm.",
            "Bonjour",
            "›",
            "suggestion-explain-project"
        ]
    );
    let strip = shown_strip(&app);
    assert!(strip.contains(" 1k/1.0M"), "{strip}");
    assert!(
        strip.contains("cost-dollars 0decimal-separator03"),
        "{strip}"
    );
}

#[test]
fn a_failed_reply_ends_with_its_notice() {
    let (mut app, receiver) = chatting(ScriptedChat {
        failure: Some(ReplyFailure::Overloaded),
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));

    until_replied(&mut app, &receiver);
    settled(&mut app);

    assert_eq!(
        flow_texts(&app),
        vec![
            "› hello",
            "reply-overloaded",
            "›",
            "suggestion-explain-project"
        ]
    );
}

#[test]
fn escape_stops_claude_at_once() {
    let (mut app, receiver) = chatting(ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bon".into())],
        waits_to_be_stopped: true,
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));
    let first = receiver.recv_timeout(PATIENCE).unwrap();
    deliver(&mut app, first, Instant::now());

    act(&mut app, named(NamedKey::Escape));
    settled(&mut app);

    assert_eq!(
        flow_texts(&app),
        vec![
            "› hello",
            "Bon",
            "reply-stopped",
            "›",
            "suggestion-explain-project"
        ]
    );
}

fn asking_chat() -> ScriptedChat {
    ScriptedChat {
        asks: vec![Permission {
            tool_id: "t1".to_owned(),
            action: Action::Change {
                path: "src/lib.rs".to_owned(),
            },
        }],
        pieces: vec![
            ReplyEvent::ToolStarted {
                id: "t1".to_owned(),
                tool: ToolUse {
                    kind: ToolKind::Edit,
                    subject: "src/lib.rs".to_owned(),
                },
            },
            ReplyEvent::ToolEnded {
                id: "t1".to_owned(),
                end: ToolEnd::Done(Detail::Changes {
                    added: 3,
                    removed: 1,
                }),
            },
        ],
        ..ScriptedChat::default()
    }
}

fn asked(app: &mut TestApp, receiver: &Receiver<Delivery>) -> Instant {
    act(app, typed("fix it"));
    act(app, named(NamedKey::Enter));
    let asked_at = Instant::now();
    deliver(app, receiver.recv_timeout(PATIENCE).unwrap(), asked_at);
    asked_at
}

#[test]
fn return_lets_claude_change_the_file_then_the_status_bar_counts_the_lines() {
    let chat = asking_chat();
    let decisions = Arc::clone(&chat.decisions);
    let (mut app, receiver) = chatting(chat);
    let asked_at = asked(&mut app, &receiver);
    let question = flow_texts(&app);

    app.handle(named(NamedKey::Enter), asked_at);
    let answered_too_soon = !decisions.lock().unwrap().is_empty();
    app.handle(
        named(NamedKey::Enter),
        asked_at + Duration::from_millis(300),
    );
    until_replied(&mut app, &receiver);
    settled(&mut app);

    assert!(
        question.contains(&"› permission-change".to_owned()),
        "{question:?}"
    );
    assert!(!answered_too_soon);
    assert_eq!(*decisions.lock().unwrap(), [Decision::Allow]);
    let strip = shown_strip(&app);
    assert!(strip.contains("+3/−1"), "{strip}");
}

#[test]
fn escape_refuses_without_stopping_the_reply() {
    let chat = asking_chat();
    let decisions = Arc::clone(&chat.decisions);
    let (mut app, receiver) = chatting(chat);
    let asked_at = asked(&mut app, &receiver);

    app.handle(
        named(NamedKey::Escape),
        asked_at + Duration::from_millis(300),
    );
    until_replied(&mut app, &receiver);
    settled(&mut app);

    assert_eq!(*decisions.lock().unwrap(), [Decision::Deny]);
    assert_eq!(
        flow_texts(&app).last().map(String::as_str),
        Some("suggestion-explain-project")
    );
    assert!(!flow_texts(&app).contains(&"reply-stopped".to_owned()));
}

fn prompts_seen(prompts: &Arc<std::sync::Mutex<Vec<String>>>, count: usize) -> Vec<String> {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let seen = prompts.lock().unwrap().clone();
        if seen.len() >= count || Instant::now() > deadline {
            return seen;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_reply_that_ends_in_the_session_shown_leaves_nothing_to_read() {
    let (mut app, receiver) = chatting(ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bonjour".into())],
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));

    until_replied(&mut app, &receiver);

    assert_eq!(app.sessions.unread(), 0);
}

#[test]
fn a_prompt_queued_during_a_reply_is_sent_once_it_ends() {
    let prompts = Arc::default();
    let (mut app, receiver) = chatting(ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bonjour".into())],
        prompts: Arc::clone(&prompts),
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));
    act(&mut app, typed("next"));
    act(&mut app, named(NamedKey::Enter));

    until_replied(&mut app, &receiver);

    assert_eq!(prompts_seen(&prompts, 2), ["hello", "next"]);
}

#[test]
fn a_note_during_a_pause_stops_the_reply_and_asks_claude_again() {
    let prompts = Arc::default();
    let (mut app, receiver) = chatting(ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bon".into())],
        waits_to_be_stopped: true,
        prompts: Arc::clone(&prompts),
        ..ScriptedChat::default()
    });
    act(&mut app, typed("hello"));
    act(&mut app, named(NamedKey::Enter));
    let first = receiver.recv_timeout(PATIENCE).unwrap();
    deliver(&mut app, first, Instant::now());

    act(&mut app, named(NamedKey::Tab));
    act(&mut app, typed("shorter"));
    act(&mut app, named(NamedKey::Enter));

    assert_eq!(prompts_seen(&prompts, 2), ["hello", "shorter"]);
    act(&mut app, named(NamedKey::Escape));
}
