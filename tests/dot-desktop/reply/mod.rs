use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use dot_api::{
    Action, ApiKey, Archive, Cost, Credential, Decision, Permissions, ReplyError, ReplyFailure,
    Text, Tuning,
};

use super::*;
use crate::keeping::tests::{MemoryArchive, MemoryJournal};
use crate::sign_in::FakeConnector;
use crate::status_bar::FixedSession;
use crate::worker::join;

const PATIENCE: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(crate) struct ScriptedChat {
    pub(crate) pieces: Vec<ReplyEvent>,
    pub(crate) failure: Option<ReplyFailure>,
    pub(crate) waits_to_be_stopped: bool,
    pub(crate) holds_until_released: bool,
    pub(crate) prompts: Arc<Mutex<Vec<String>>>,
    pub(crate) asks: Vec<Permission>,
    pub(crate) decisions: Arc<Mutex<Vec<Decision>>>,
    pub(crate) panics: bool,
    pub(crate) spent: Option<Reply>,
    pub(crate) resets: Arc<AtomicU32>,
    pub(crate) ended: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) settles: Arc<AtomicU32>,
    pub(crate) tunings: Arc<Mutex<Vec<Tuning>>>,
    pub(crate) settling: Settling,
    pub(crate) released: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) settle_stopped: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) remembers: Vec<u8>,
    pub(crate) recalled: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Settling {
    #[default]
    AtOnce,
    UntilReleased,
    Crashing,
}

impl Chat for ScriptedChat {
    fn reply(
        &mut self,
        prompt: Prompt,
        _credential: &Credential,
        cancellation: &Cancellation,
        permissions: &dyn Permissions,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        self.prompts.lock().unwrap().push(prompt.text.clone());
        for permission in &self.asks {
            let decision = permissions.ask(permission, cancellation);
            self.decisions.lock().unwrap().push(decision);
        }
        for piece in &self.pieces {
            on_event(piece.clone());
        }
        while self.waits_to_be_stopped && !cancellation.is_cancelled() {
            thread::sleep(Duration::from_millis(1));
        }
        while self.holds_until_released && !self.released.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(1));
        }
        if self.waits_to_be_stopped {
            thread::sleep(Duration::from_millis(20));
        }
        self.ended.store(true, Ordering::Relaxed);
        assert!(!self.panics, "the scripted chat fails");
        let spent = self.spent.unwrap_or(Reply {
            context_tokens: 1_200,
            cost: Cost::from_micro_dollars(30_000),
            ..Reply::default()
        });
        if cancellation.is_cancelled() {
            return Err(ReplyError::new(ReplyFailure::Stopped).with_spent(spent));
        }
        if let Some(failure) = self.failure {
            return Err(ReplyError::new(failure).with_spent(spent));
        }
        Ok(spent)
    }

    fn reset(&mut self) {
        self.resets.fetch_add(1, Ordering::Relaxed);
    }

    fn tune(&mut self, tuning: Tuning) {
        self.tunings.lock().unwrap().push(tuning);
    }

    fn remember(&self, memory: &mut dyn std::io::Write) -> std::io::Result<()> {
        memory.write_all(&self.remembers)
    }

    fn recall(&mut self, memory: Vec<u8>) {
        self.recalled.lock().unwrap().push(memory);
    }

    fn settle(&mut self, cancellation: &Cancellation) {
        self.settles.fetch_add(1, Ordering::Relaxed);
        assert!(
            self.settling != Settling::Crashing,
            "the scripted chat fails to settle"
        );
        while self.settling == Settling::UntilReleased
            && !self.released.load(Ordering::Relaxed)
            && !cancellation.is_cancelled()
        {
            thread::sleep(Duration::from_millis(1));
        }
        if cancellation.is_cancelled() {
            self.settle_stopped.store(true, Ordering::Relaxed);
        }
    }
}

pub(crate) fn wait_until(what: &str, happened: impl Fn() -> bool) {
    let started = std::time::Instant::now();
    while !happened() {
        assert!(started.elapsed() < PATIENCE, "{what}");
        thread::sleep(Duration::from_millis(1));
    }
}

pub(crate) type TestReplies =
    Replies<FakeConnector, FixedSession, ScriptedChat, Sender<ReplyNews>, MemoryJournal>;

pub(crate) fn unkept() -> Recorder<MemoryJournal> {
    let archive = MemoryArchive::default();
    Recorder::new(archive.journal(dot_api::KeptId::new(1)), Clock::default())
}

pub(crate) fn asked(text: &str) -> Prompt {
    Prompt {
        text: text.to_owned(),
        images: Vec::new(),
    }
}

pub(crate) fn connected() -> FakeConnector {
    FakeConnector {
        credential: Some(Credential::ApiKey(
            ApiKey::parse("sk-ant-api03-0123456789abcdef").unwrap(),
        )),
        ..FakeConnector::default()
    }
}

fn replies(connector: FakeConnector, chat: ScriptedChat) -> (TestReplies, Receiver<ReplyNews>) {
    replies_in(PermissionMode::Ask, connector, chat)
}

fn replies_in(
    mode: PermissionMode,
    connector: FakeConnector,
    chat: ScriptedChat,
) -> (TestReplies, Receiver<ReplyNews>) {
    let (replies, _session, receiver) = replies_with_session(mode, connector, chat);
    (replies, receiver)
}

pub(crate) fn replies_with_session(
    mode: PermissionMode,
    connector: FakeConnector,
    chat: ScriptedChat,
) -> (TestReplies, Arc<FixedSession>, Receiver<ReplyNews>) {
    let (sender, receiver) = mpsc::channel();
    let session = Arc::new(FixedSession::default());
    let replies = Replies::new(
        Arc::new(connector),
        Arc::clone(&session),
        (chat, unkept()),
        mode,
        sender,
    );
    (replies, session, receiver)
}

pub(crate) fn heard_until_the_end(
    replies: &mut TestReplies,
    receiver: &Receiver<ReplyNews>,
) -> Vec<News> {
    let mut heard = Vec::new();
    loop {
        let news = receiver.recv_timeout(PATIENCE).unwrap();
        let Some(news) = replies.hear(news) else {
            continue;
        };
        let ended = matches!(news, News::Ended(_));
        heard.push(news);
        if ended {
            return heard;
        }
    }
}

#[test]
fn streams_the_reply_then_its_end() {
    let chat = ScriptedChat {
        pieces: vec![
            ReplyEvent::Thinking("Hm.".into()),
            ReplyEvent::Text("Bon".into()),
            ReplyEvent::Text("jour".into()),
        ],
        ..ScriptedChat::default()
    };
    let prompts = Arc::clone(&chat.prompts);
    let (mut replies, receiver) = replies(connected(), chat);

    replies.ask(asked("Salut"), Opened::default());
    let heard = heard_until_the_end(&mut replies, &receiver);

    assert_eq!(
        heard,
        vec![
            News::Piece(ReplyEvent::Thinking("Hm.".into())),
            News::Piece(ReplyEvent::Text("Bon".into())),
            News::Piece(ReplyEvent::Text("jour".into())),
            News::Ended(Ok(Reply {
                context_tokens: 1_200,
                cost: Cost::from_micro_dollars(30_000),
                ..Reply::default()
            })),
        ]
    );
    assert_eq!(*prompts.lock().unwrap(), ["Salut"]);
    assert!(!replies.stop());
}

#[test]
fn ends_with_the_text_of_a_failure() {
    let chat = ScriptedChat {
        failure: Some(ReplyFailure::Overloaded),
        ..ScriptedChat::default()
    };
    let (mut replies, receiver) = replies(connected(), chat);

    replies.ask(asked("Salut"), Opened::default());
    let heard = heard_until_the_end(&mut replies, &receiver);

    assert_eq!(
        heard,
        vec![News::Ended(Err(Problem {
            text: Text::ReplyOverloaded,
            cause: None,
        }))]
    );
}

#[test]
fn ends_with_the_connection_failure_when_no_credential_is_handed_out() {
    let chat = ScriptedChat::default();
    let prompts = Arc::clone(&chat.prompts);
    let (mut replies, receiver) = replies(FakeConnector::default(), chat);

    replies.ask(asked("Salut"), Opened::default());
    let heard = heard_until_the_end(&mut replies, &receiver);

    assert_eq!(
        heard,
        vec![News::Ended(Err(Problem {
            text: Text::SignInNotConnected,
            cause: None,
        }))]
    );
    assert!(prompts.lock().unwrap().is_empty());
}

#[test]
fn stopping_ends_the_reply_and_silences_it() {
    let chat = ScriptedChat {
        pieces: vec![ReplyEvent::Text("Bon".into())],
        waits_to_be_stopped: true,
        ..ScriptedChat::default()
    };
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    let first = receiver.recv_timeout(PATIENCE).unwrap();

    assert!(replies.stop());
    stop_and_wait(&mut replies);

    assert_eq!(replies.hear(first), None);
    let rest: Vec<ReplyNews> = receiver.try_iter().collect();
    assert!(rest.iter().all(|news| matches!(news.news, News::Ended(_))));
    assert_eq!(
        rest.into_iter()
            .filter_map(|news| replies.hear(news))
            .count(),
        0
    );
    assert!(!replies.stop());
}

#[test]
fn ignores_news_of_an_earlier_reply() {
    let chat = ScriptedChat {
        pieces: vec![ReplyEvent::Text("old".into())],
        ..ScriptedChat::default()
    };
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("first"), Opened::default());
    let stale = receiver.recv_timeout(PATIENCE).unwrap();
    heard_until_the_end(&mut replies, &receiver);

    replies.ask(asked("second"), Opened::default());

    assert_eq!(replies.hear(stale), None);
}

fn run(command: &str) -> Permission {
    Permission {
        tool_id: "t1".to_owned(),
        action: Action::Run {
            command: command.to_owned(),
        },
    }
}

fn asking_chat() -> ScriptedChat {
    ScriptedChat {
        asks: vec![run("ls")],
        pieces: vec![ReplyEvent::Text("Done".into())],
        ..ScriptedChat::default()
    }
}

#[test]
fn hands_the_question_over_then_the_user_s_answer_back() {
    let chat = asking_chat();
    let decisions = Arc::clone(&chat.decisions);
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("list"), Opened::default());

    let question = replies.hear(receiver.recv_timeout(PATIENCE).unwrap());
    let answered = replies.answer(Answer::Allow);
    let heard = heard_until_the_end(&mut replies, &receiver);

    assert_eq!(question, Some(News::Asked(run("ls"))));
    assert!(answered);
    assert!(!replies.answer(Answer::Allow));
    assert_eq!(*decisions.lock().unwrap(), [Decision::Allow]);
    assert_eq!(heard[0], News::Piece(ReplyEvent::Text("Done".into())));
}

#[test]
fn bypass_runs_the_reply_without_a_question() {
    let chat = asking_chat();
    let decisions = Arc::clone(&chat.decisions);
    let (mut replies, receiver) = replies_in(PermissionMode::Bypass, connected(), chat);
    replies.ask(asked("list"), Opened::default());

    let heard = heard_until_the_end(&mut replies, &receiver);

    assert_eq!(*decisions.lock().unwrap(), [Decision::Allow]);
    assert!(!heard.iter().any(|news| matches!(news, News::Asked(_))));
    assert_eq!(heard[0], News::Piece(ReplyEvent::Text("Done".into())));
    assert!(!replies.answer(Answer::Allow));
}

#[test]
fn stopping_a_reply_refuses_its_question_and_forgets_it() {
    let chat = ScriptedChat {
        waits_to_be_stopped: true,
        ..asking_chat()
    };
    let decisions = Arc::clone(&chat.decisions);
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("list"), Opened::default());
    replies.hear(receiver.recv_timeout(PATIENCE).unwrap());

    assert!(replies.stop());
    stop_and_wait(&mut replies);

    assert!(!replies.answer(Answer::Allow));
    assert_eq!(*decisions.lock().unwrap(), [Decision::Deny]);
}

#[test]
fn an_answer_with_no_question_does_nothing() {
    let (mut replies, _receiver) = replies(connected(), ScriptedChat::default());

    assert!(!replies.answer(Answer::Deny));
}

#[test]
fn dropping_the_replies_stops_a_reply_under_way_and_its_thread() {
    let chat = ScriptedChat {
        waits_to_be_stopped: true,
        ..ScriptedChat::default()
    };
    let prompts = Arc::clone(&chat.prompts);
    let ended = Arc::clone(&chat.ended);
    let (mut replies, _receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    let started = std::time::Instant::now();
    while prompts.lock().unwrap().is_empty() {
        assert!(started.elapsed() < PATIENCE, "the reply never started");
        thread::sleep(Duration::from_millis(1));
    }

    drop(replies);

    assert!(
        ended.load(Ordering::Relaxed),
        "the reply thread was left running"
    );
    assert!(started.elapsed() < PATIENCE);
}
fn settling_chat() -> ScriptedChat {
    ScriptedChat {
        settling: Settling::UntilReleased,
        ..ScriptedChat::default()
    }
}

#[test]
fn a_new_conversation_stops_what_dot_was_doing_between_two_prompts() {
    let chat = settling_chat();
    let (settles, stopped, resets) = (
        Arc::clone(&chat.settles),
        Arc::clone(&chat.settle_stopped),
        Arc::clone(&chat.resets),
    );
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    heard_until_the_end(&mut replies, &receiver);
    wait_until("nothing was settled", || {
        settles.load(Ordering::Relaxed) == 1
    });

    replies.start_over();

    wait_until("the conversation never restarted", || {
        resets.load(Ordering::Relaxed) == 1
    });
    assert!(stopped.load(Ordering::Relaxed));
}

#[test]
fn a_new_prompt_waits_for_what_dot_is_doing_between_two_prompts() {
    let chat = settling_chat();
    let (settles, stopped, released, prompts) = (
        Arc::clone(&chat.settles),
        Arc::clone(&chat.settle_stopped),
        Arc::clone(&chat.released),
        Arc::clone(&chat.prompts),
    );
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    heard_until_the_end(&mut replies, &receiver);
    wait_until("nothing was settled", || {
        settles.load(Ordering::Relaxed) == 1
    });

    replies.ask(asked("Encore"), Opened::default());
    thread::sleep(Duration::from_millis(30));
    let asked_meanwhile = prompts.lock().unwrap().len();
    released.store(true, Ordering::Relaxed);
    heard_until_the_end(&mut replies, &receiver);

    assert_eq!(asked_meanwhile, 1);
    assert_eq!(*prompts.lock().unwrap(), ["Salut", "Encore"]);
    assert!(!stopped.load(Ordering::Relaxed));
}

#[test]
fn closing_stops_what_dot_was_doing_between_two_prompts() {
    let chat = settling_chat();
    let (settles, stopped) = (Arc::clone(&chat.settles), Arc::clone(&chat.settle_stopped));
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    heard_until_the_end(&mut replies, &receiver);
    wait_until("nothing was settled", || {
        settles.load(Ordering::Relaxed) == 1
    });

    stop_and_wait(&mut replies);

    assert!(stopped.load(Ordering::Relaxed));
}

#[test]
fn dropping_the_replies_stops_what_dot_was_doing_between_two_prompts() {
    let chat = settling_chat();
    let (settles, stopped) = (Arc::clone(&chat.settles), Arc::clone(&chat.settle_stopped));
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    heard_until_the_end(&mut replies, &receiver);
    wait_until("nothing was settled", || {
        settles.load(Ordering::Relaxed) == 1
    });

    drop(replies);

    wait_until("the settling never stopped", || {
        stopped.load(Ordering::Relaxed)
    });
}

pub(crate) fn news_of(text: &str) -> ReplyNews {
    ReplyNews {
        id: 1,
        news: News::Piece(ReplyEvent::Text(text.to_owned())),
    }
}

fn stop_and_wait(replies: &mut TestReplies) {
    if let Some(thread) = replies.stop_all() {
        join(thread);
    }
}

#[test]
fn closing_after_a_second_prompt_stops_what_dot_was_doing_between_the_two() {
    let chat = settling_chat();
    let (settles, stopped) = (Arc::clone(&chat.settles), Arc::clone(&chat.settle_stopped));
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    heard_until_the_end(&mut replies, &receiver);
    wait_until("nothing was settled", || {
        settles.load(Ordering::Relaxed) == 1
    });
    replies.ask(asked("Encore"), Opened::default());
    let started = std::time::Instant::now();

    stop_and_wait(&mut replies);

    assert!(stopped.load(Ordering::Relaxed));
    assert!(started.elapsed() < PATIENCE);
}

#[test]
fn closing_lets_the_reply_thread_go_without_waiting_for_it() {
    let chat = ScriptedChat {
        holds_until_released: true,
        ..ScriptedChat::default()
    };
    let (ended, released, prompts) = (
        Arc::clone(&chat.ended),
        Arc::clone(&chat.released),
        Arc::clone(&chat.prompts),
    );
    let (mut replies, _receiver) = replies(connected(), chat);
    replies.ask(asked("Salut"), Opened::default());
    wait_until("the reply never started", || {
        !prompts.lock().unwrap().is_empty()
    });

    let thread = replies.stop_all().expect("the reply thread was running");

    assert!(!thread.is_finished());
    released.store(true, Ordering::Relaxed);
    join(thread);
    assert!(ended.load(Ordering::Relaxed));
}

#[test]
fn a_switch_to_bypass_runs_the_next_reply_without_a_question() {
    let chat = asking_chat();
    let decisions = Arc::clone(&chat.decisions);
    let (mut replies, receiver) = replies(connected(), chat);

    replies.switch_to(PermissionMode::Bypass);
    replies.ask(asked("list"), Opened::default());
    let heard = heard_until_the_end(&mut replies, &receiver);

    assert_eq!(*decisions.lock().unwrap(), [Decision::Allow]);
    assert!(!heard.iter().any(|news| matches!(news, News::Asked(_))));
}

#[test]
fn a_switch_to_ask_puts_the_next_action_to_the_user() {
    let chat = asking_chat();
    let (mut replies, receiver) = replies_in(PermissionMode::Bypass, connected(), chat);

    replies.switch_to(PermissionMode::Ask);
    replies.ask(asked("list"), Opened::default());

    let question = replies.hear(receiver.recv_timeout(PATIENCE).unwrap());
    assert_eq!(question, Some(News::Asked(run("ls"))));
    assert!(replies.answer(Answer::Allow));
    heard_until_the_end(&mut replies, &receiver);
}

#[test]
fn tunes_the_conversation_before_its_first_reply() {
    let chat = ScriptedChat::default();
    let tunings = Arc::clone(&chat.tunings);
    let (mut replies, _receiver) = replies(connected(), chat);
    let tuning = Tuning {
        model: dot_api::Model::HAIKU_5_5,
        ..crate::settings::tests::test_settings().tuning
    };

    replies.tune(tuning);

    assert_eq!(*tunings.lock().unwrap(), [tuning]);
}

#[test]
fn tunes_the_conversation_on_its_thread_once_it_has_replied() {
    let chat = ScriptedChat::default();
    let tunings = Arc::clone(&chat.tunings);
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("hello"), Opened::default());
    heard_until_the_end(&mut replies, &receiver);
    let tuning = Tuning {
        effort: dot_api::Effort::Max,
        ..crate::settings::tests::test_settings().tuning
    };

    replies.tune(tuning);

    wait_until("the tuning reaches the conversation", || {
        *tunings.lock().unwrap() == [tuning]
    });
}
