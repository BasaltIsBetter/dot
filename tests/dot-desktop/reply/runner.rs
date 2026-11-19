use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Duration;

use dot_api::{Cost, PermissionMode, Text};

use super::*;
use crate::reply::tests::{ScriptedChat, asked, connected};
use crate::sign_in::FakeConnector;
use crate::status_bar::FixedSession;

const PATIENCE: Duration = Duration::from_secs(5);

fn started(chat: ScriptedChat) -> (Runner, Arc<FixedSession>, Receiver<ReplyNews>) {
    let (mailbox, received) = mpsc::channel();
    let session = Arc::new(FixedSession::default());
    let crew: Crew<FakeConnector, FixedSession, Sender<ReplyNews>> = Crew {
        connector: Arc::new(connected()),
        session: Arc::clone(&session),
        asking: Asking::new(PermissionMode::Bypass),
        mailbox,
    };
    (Runner::start(chat, crew), session, received)
}

fn reply(id: u64, cancellation: &Cancellation) -> Job {
    Job::Reply {
        id,
        prompt: asked("Salut"),
        cancellation: cancellation.clone(),
    }
}

fn end_of(received: &Receiver<ReplyNews>, id: u64) -> News {
    loop {
        let news = received.recv_timeout(PATIENCE).unwrap();
        if news.id == id && matches!(news.news, News::Ended(_)) {
            return news.news;
        }
    }
}

#[test]
fn a_reply_that_panics_ends_in_a_crash_and_starts_the_conversation_afresh() {
    let chat = ScriptedChat {
        panics: true,
        ..ScriptedChat::default()
    };
    let resets = Arc::clone(&chat.resets);
    let (runner, session, received) = started(chat);
    session.record(&Reply {
        context_tokens: 9_000,
        ..Reply::default()
    });

    runner.send(reply(0, &Cancellation::new()));

    assert_eq!(
        end_of(&received, 0),
        News::Ended(Err(Problem {
            text: Text::ReplyCrashed,
            cause: Some("the scripted chat fails".to_owned()),
        }))
    );
    assert_eq!(resets.load(Ordering::Relaxed), 1);
    assert_eq!(session.spending().usage.context_tokens, 0);
}

#[test]
fn keeps_answering_after_a_crash() {
    let chat = ScriptedChat {
        panics: true,
        ..ScriptedChat::default()
    };
    let (runner, _session, received) = started(chat);

    runner.send(reply(0, &Cancellation::new()));
    runner.send(reply(1, &Cancellation::new()));

    assert!(matches!(end_of(&received, 0), News::Ended(Err(_))));
    assert!(matches!(end_of(&received, 1), News::Ended(Err(_))));
}

#[test]
fn records_what_a_stopped_reply_spent_and_still_says_it_ended() {
    let chat = ScriptedChat {
        waits_to_be_stopped: true,
        spent: Some(Reply {
            context_tokens: 4_000,
            cost: Cost::from_micro_dollars(70_000),
            ..Reply::default()
        }),
        ..ScriptedChat::default()
    };
    let prompts = Arc::clone(&chat.prompts);
    let (runner, session, received) = started(chat);
    let cancellation = Cancellation::new();

    runner.send(reply(0, &cancellation));
    let started_at = std::time::Instant::now();
    while prompts.lock().unwrap().is_empty() {
        assert!(started_at.elapsed() < PATIENCE, "the reply never started");
        std::thread::sleep(Duration::from_millis(1));
    }
    cancellation.cancel();

    assert!(matches!(end_of(&received, 0), News::Ended(Err(_))));
    let usage = session.spending().usage;
    assert_eq!(usage.cost, Cost::from_micro_dollars(70_000));
    assert_eq!(usage.context_tokens, 4_000);
}

#[test]
fn counts_the_lines_tools_changed_in_the_session() {
    let chat = ScriptedChat {
        pieces: vec![ReplyEvent::ToolEnded {
            id: "t1".to_owned(),
            end: ToolEnd::Done(Detail::Changes {
                added: 3,
                removed: 1,
            }),
        }],
        ..ScriptedChat::default()
    };
    let (runner, session, received) = started(chat);

    runner.send(reply(0, &Cancellation::new()));
    end_of(&received, 0);

    let usage = session.spending().usage;
    assert_eq!((usage.lines_added, usage.lines_removed), (3, 1));
}

#[test]
fn a_reply_stopped_before_its_turn_never_reaches_claude() {
    let chat = ScriptedChat::default();
    let prompts = Arc::clone(&chat.prompts);
    let (runner, _session, received) = started(chat);
    let cancellation = Cancellation::new();
    cancellation.cancel();

    runner.send(reply(0, &cancellation));

    assert_eq!(
        end_of(&received, 0),
        News::Ended(Err(Problem::plain(Text::ReplyStopped)))
    );
    assert!(prompts.lock().unwrap().is_empty());
}

#[test]
fn starting_over_forgets_the_conversation_and_empties_the_context() {
    let chat = ScriptedChat::default();
    let resets = Arc::clone(&chat.resets);
    let (runner, session, received) = started(chat);
    runner.send(reply(0, &Cancellation::new()));
    end_of(&received, 0);

    let cancelled = Cancellation::new();
    cancelled.cancel();

    runner.send(Job::Reset);
    runner.send(reply(1, &cancelled));
    end_of(&received, 1);

    assert_eq!(resets.load(Ordering::Relaxed), 1);
    assert_eq!(session.spending().usage.context_tokens, 0);
    assert_eq!(
        session.spending().usage.cost,
        Cost::from_micro_dollars(30_000)
    );
}

#[test]
fn a_stopped_runner_takes_no_more_jobs() {
    let chat = ScriptedChat::default();
    let prompts = Arc::clone(&chat.prompts);
    let (mut runner, _session, received) = started(chat);

    runner.stop();
    runner.send(reply(0, &Cancellation::new()));

    assert!(received.try_recv().is_err());
    assert!(prompts.lock().unwrap().is_empty());
}
