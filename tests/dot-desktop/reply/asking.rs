use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use dot_api::Action;

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);
const PROMPT: Duration = Duration::from_millis(500);

fn run(command: &str) -> Permission {
    Permission {
        tool_id: "t1".to_owned(),
        action: Action::Run {
            command: command.to_owned(),
        },
    }
}

fn change(path: &str) -> Permission {
    Permission {
        tool_id: "t2".to_owned(),
        action: Action::Change {
            path: path.to_owned(),
        },
    }
}

fn ask_in_the_background(
    asking: &Asking,
    permission: Permission,
    cancellation: &Cancellation,
) -> (Receiver<Decision>, Receiver<ReplyNews>) {
    ask_in_mode(PermissionMode::Ask, asking, permission, cancellation)
}

fn ask_in_mode(
    mode: PermissionMode,
    asking: &Asking,
    permission: Permission,
    cancellation: &Cancellation,
) -> (Receiver<Decision>, Receiver<ReplyNews>) {
    let (sender, receiver) = mpsc::channel();
    let (decide, decision) = mpsc::channel();
    let asking = asking.clone();
    let cancellation = cancellation.clone();
    thread::spawn(move || {
        let deliver = move |news: ReplyNews| sender.send(news).unwrap();
        let asker = Asker {
            id: 7,
            mode,
            asking,
            deliver: &deliver,
        };
        decide.send(asker.ask(&permission, &cancellation)).unwrap();
    });
    (decision, receiver)
}

fn decided(asking: &Asking, permission: &Permission, answer: Answer) -> Decision {
    let (asked, news) = ask_in_the_background(asking, permission.clone(), &Cancellation::new());
    let heard = news.recv_timeout(PATIENCE).unwrap();
    assert_eq!(heard.id, 7);
    assert_eq!(heard.news, News::Asked(permission.clone()));
    assert!(asking.answer(answer, permission));
    asked.recv_timeout(PATIENCE).unwrap()
}

fn asks_again(asking: &Asking, permission: Permission) -> bool {
    let cancellation = Cancellation::new();
    let (asked, news) = ask_in_the_background(asking, permission, &cancellation);
    let asked_again = news.recv_timeout(PROMPT).is_ok();
    cancellation.cancel();
    asked.recv_timeout(PATIENCE).unwrap();
    asked_again
}

#[test]
fn hands_over_the_question_then_waits_for_the_user() {
    let asking = Asking::default();

    assert_eq!(decided(&asking, &run("ls"), Answer::Allow), Decision::Allow);
    assert_eq!(decided(&asking, &run("ls"), Answer::Deny), Decision::Deny);
}

#[test]
fn yes_always_covers_what_the_action_grants_until_the_window_closes() {
    let asking = Asking::default();

    assert_eq!(
        decided(&asking, &run("cargo test"), Answer::Always),
        Decision::Allow
    );
    assert_eq!(
        decided(&asking, &change("src/a.rs"), Answer::Always),
        Decision::Allow
    );

    assert!(!asks_again(&asking, run("cargo test")));
    assert!(!asks_again(&asking, change("src/b.rs")));
    assert!(asks_again(&asking, run("cargo test --release")));
}

#[test]
fn a_plain_yes_covers_nothing_more() {
    let asking = Asking::default();

    decided(&asking, &run("ls"), Answer::Allow);

    assert!(asks_again(&asking, run("ls")));
}

#[test]
fn a_stopped_reply_refuses_at_once() {
    let asking = Asking::default();
    let cancellation = Cancellation::new();
    let (asked, news) = ask_in_the_background(&asking, run("ls"), &cancellation);
    news.recv_timeout(PATIENCE).unwrap();

    let stopped = Instant::now();
    cancellation.cancel();

    assert_eq!(asked.recv_timeout(PATIENCE), Ok(Decision::Deny));
    assert!(stopped.elapsed() < PROMPT);
}

#[test]
fn refuses_when_nobody_can_answer_any_more() {
    let asking = Asking::default();
    let (asked, news) = ask_in_the_background(&asking, run("ls"), &Cancellation::new());
    news.recv_timeout(PATIENCE).unwrap();

    asking.shared().waiting.take();

    assert_eq!(asked.recv_timeout(PATIENCE), Ok(Decision::Deny));
}

#[test]
fn an_answer_with_no_question_waiting_does_nothing() {
    let asking = Asking::default();

    assert!(!asking.answer(Answer::Always, &run("ls")));
    assert_eq!(asking.shared().grants, []);
}

#[test]
fn waits_as_long_as_the_user_takes_to_answer() {
    let asking = Asking::default();
    let permission = run("ls");
    let (asked, news) = ask_in_the_background(&asking, permission.clone(), &Cancellation::new());
    news.recv_timeout(PATIENCE).unwrap();

    thread::sleep(Duration::from_millis(200));
    assert!(asking.answer(Answer::Allow, &permission));

    assert_eq!(asked.recv_timeout(PATIENCE), Ok(Decision::Allow));
}

fn bypassed(permission: &Permission, cancellation: &Cancellation) -> (Decision, usize) {
    let (decision, news) = ask_in_mode(
        PermissionMode::Bypass,
        &Asking::default(),
        permission.clone(),
        cancellation,
    );
    let decision = decision.recv_timeout(PATIENCE).unwrap();
    (decision, news.try_iter().count())
}

#[test]
fn bypass_allows_every_action_without_asking() {
    let cancellation = Cancellation::new();

    for permission in [run("rm -rf build"), change("/etc/hosts")] {
        let (decision, questions) = bypassed(&permission, &cancellation);

        assert_eq!(decision, Decision::Allow);
        assert_eq!(questions, 0);
    }
}

#[test]
fn bypass_still_refuses_a_stopped_reply() {
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let (decision, questions) = bypassed(&run("ls"), &cancellation);

    assert_eq!(decision, Decision::Deny);
    assert_eq!(questions, 0);
}
