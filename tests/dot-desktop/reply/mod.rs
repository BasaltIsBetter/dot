use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use dot_api::{
    Action, ApiKey, Cost, Credential, Decision, Permissions, ReplyError, ReplyFailure, Text,
};

use super::*;
use crate::sign_in::FakeConnector;

const PATIENCE: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(crate) struct ScriptedChat {
    pub(crate) pieces: Vec<ReplyEvent>,
    pub(crate) failure: Option<ReplyFailure>,
    pub(crate) waits_to_be_stopped: bool,
    pub(crate) prompts: Arc<Mutex<Vec<String>>>,
    pub(crate) asks: Vec<Permission>,
    pub(crate) decisions: Arc<Mutex<Vec<Decision>>>,
}

impl Chat for ScriptedChat {
    fn reply(
        &self,
        prompt: &Prompt,
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
        if let Some(failure) = self.failure {
            return Err(ReplyError::new(failure));
        }
        Ok(Reply {
            context_tokens: 1_200,
            cost: Cost::from_micro_dollars(30_000),
            ..Reply::default()
        })
    }
}

fn asked(text: &str) -> Prompt {
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

fn replies(
    connector: FakeConnector,
    chat: ScriptedChat,
) -> (
    Replies<FakeConnector, ScriptedChat, Sender<ReplyNews>>,
    Receiver<ReplyNews>,
) {
    replies_in(PermissionMode::Ask, connector, chat)
}

fn replies_in(
    mode: PermissionMode,
    connector: FakeConnector,
    chat: ScriptedChat,
) -> (
    Replies<FakeConnector, ScriptedChat, Sender<ReplyNews>>,
    Receiver<ReplyNews>,
) {
    let (sender, receiver) = mpsc::channel();
    (
        Replies::new(Arc::new(connector), chat, mode, sender),
        receiver,
    )
}

fn heard_until_the_end(
    replies: &mut Replies<FakeConnector, ScriptedChat, Sender<ReplyNews>>,
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

    replies.ask(asked("Salut"));
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

    replies.ask(asked("Salut"));
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

    replies.ask(asked("Salut"));
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
    replies.ask(asked("Salut"));
    let first = receiver.recv_timeout(PATIENCE).unwrap();

    assert!(replies.stop());
    replies.stop_all();

    assert_eq!(replies.hear(first), None);
    assert!(receiver.try_recv().is_err());
    assert!(!replies.stop());
}

#[test]
fn ignores_news_of_an_earlier_reply() {
    let chat = ScriptedChat {
        pieces: vec![ReplyEvent::Text("old".into())],
        ..ScriptedChat::default()
    };
    let (mut replies, receiver) = replies(connected(), chat);
    replies.ask(asked("first"));
    let stale = receiver.recv_timeout(PATIENCE).unwrap();
    heard_until_the_end(&mut replies, &receiver);

    replies.ask(asked("second"));

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
    replies.ask(asked("list"));

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
    replies.ask(asked("list"));

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
    replies.ask(asked("list"));
    replies.hear(receiver.recv_timeout(PATIENCE).unwrap());

    assert!(replies.stop());
    replies.stop_all();

    assert!(!replies.answer(Answer::Allow));
    assert_eq!(*decisions.lock().unwrap(), [Decision::Deny]);
}

#[test]
fn an_answer_with_no_question_does_nothing() {
    let (mut replies, _receiver) = replies(connected(), ScriptedChat::default());

    assert!(!replies.answer(Answer::Deny));
}
