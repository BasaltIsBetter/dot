use std::time::Duration;

use std::sync::Arc;
use std::sync::mpsc::Receiver;

use dot_api::{Action, Decision, Detail, Permission, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::keyboard::NamedKey;

use super::*;
use crate::app::tests::{
    TestApp, act, chatting, named, pane_texts, settled, shown_strip, typed, until_replied,
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

    assert_eq!(pane_texts(&app, 0), vec!["Hm.", "Bonjour"]);
    let strip = shown_strip(&app);
    assert!(strip.contains("1k"), "{strip}");
    assert!(strip.contains("0.03"), "{strip}");
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

    assert_eq!(pane_texts(&app, 0), vec!["reply-overloaded"]);
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
    app.deliver(first, Instant::now());

    act(&mut app, named(NamedKey::Escape));
    settled(&mut app);

    assert_eq!(pane_texts(&app, 0), vec!["Bon", "reply-stopped"]);
    assert_eq!(pane_texts(&app, 1), vec!["› ", "_", "prompter-placeholder"]);
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
    app.deliver(receiver.recv_timeout(PATIENCE).unwrap(), asked_at);
    asked_at
}

#[test]
fn return_lets_claude_change_the_file_then_the_status_bar_counts_the_lines() {
    let chat = asking_chat();
    let decisions = Arc::clone(&chat.decisions);
    let (mut app, receiver) = chatting(chat);
    let asked_at = asked(&mut app, &receiver);
    let question = pane_texts(&app, 1);

    app.handle(named(NamedKey::Enter), asked_at);
    let answered_too_soon = !decisions.lock().unwrap().is_empty();
    app.handle(
        named(NamedKey::Enter),
        asked_at + Duration::from_millis(300),
    );
    until_replied(&mut app, &receiver);
    settled(&mut app);

    assert_eq!(question[0], "› permission-change");
    assert!(!answered_too_soon);
    assert_eq!(*decisions.lock().unwrap(), [Decision::Allow]);
    assert_eq!(pane_texts(&app, 0), vec!["tool-edit src/lib.rs  +3 −1"]);
    let strip = shown_strip(&app);
    assert!(strip.contains("+3 −1"), "{strip}");
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
        pane_texts(&app, 1).last().map(String::as_str),
        Some("prompter-placeholder")
    );
    assert!(!pane_texts(&app, 0).contains(&"reply-stopped".to_owned()));
}
