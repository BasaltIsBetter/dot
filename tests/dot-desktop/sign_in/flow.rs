use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use dot_api::{ApiKey, ConnectError, ConnectFailure, Credential, Secret};

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);
const KEY: &str = "sk-ant-api03-0123456789abcdef";

#[derive(Default)]
pub(crate) struct FakeConnector {
    pub(crate) connected: Option<Connection>,
    pub(crate) credential: Option<Credential>,
    pub(crate) claude_code_cancelled: Arc<AtomicBool>,
    pub(crate) forgotten: Arc<AtomicBool>,
    pub(crate) refuses_to_forget: bool,
}

impl Connector for FakeConnector {
    fn current(&self, _cancellation: &Cancellation) -> Result<Option<Connection>, ConnectError> {
        Ok(self.connected)
    }

    fn connect_with_api_key(
        &self,
        key: &ApiKey,
        _cancellation: &Cancellation,
    ) -> Result<(), ConnectError> {
        if key.expose() == KEY {
            return Ok(());
        }
        Err(ConnectError::new(ConnectFailure::KeyRejected))
    }

    fn connect_with_claude_code(&self, cancellation: &Cancellation) -> Result<(), ConnectError> {
        while !cancellation.is_cancelled() {
            thread::sleep(Duration::from_millis(1));
        }
        self.claude_code_cancelled.store(true, Ordering::Relaxed);
        Err(ConnectError::new(ConnectFailure::ClaudeCodeFailed))
    }

    fn connect_with_anthropic_account(
        &self,
        _cancellation: &Cancellation,
    ) -> Result<(), ConnectError> {
        Ok(())
    }

    fn credential(&self, _cancellation: &Cancellation) -> Result<Credential, ConnectError> {
        self.credential
            .clone()
            .ok_or_else(|| ConnectError::new(ConnectFailure::NotConnected))
    }

    fn forget(&self) -> Result<(), ConnectError> {
        if self.refuses_to_forget {
            return Err(ConnectError::new(ConnectFailure::StorageFailed));
        }
        self.forgotten.store(true, Ordering::Relaxed);
        Ok(())
    }
}

fn sign_in(connector: FakeConnector) -> (SignIn<FakeConnector, Sender<Report>>, Receiver<Report>) {
    let (sender, receiver) = mpsc::channel();
    (SignIn::new(Arc::new(connector), sender), receiver)
}

fn settle(sign_in: &mut SignIn<FakeConnector, Sender<Report>>, receiver: &Receiver<Report>) {
    sign_in.report(receiver.recv_timeout(PATIENCE).unwrap());
}

#[test]
fn checks_the_connection_at_start() {
    let connector = FakeConnector {
        connected: Some(Connection::ClaudeCode),
        ..FakeConnector::default()
    };
    let (mut sign_in, receiver) = sign_in(connector);

    sign_in.start();
    settle(&mut sign_in, &receiver);

    assert_eq!(sign_in.connection(), Some(Connection::ClaudeCode));
}

#[test]
fn checks_a_key_in_the_background() {
    let (mut sign_in, receiver) = sign_in(FakeConnector::default());
    sign_in.start();
    settle(&mut sign_in, &receiver);

    sign_in.input(Input::Press(Connection::ApiKey));
    sign_in.input(Input::Paste(Secret::new(KEY)));
    sign_in.input(Input::Confirm);
    assert!(matches!(sign_in.screen, Screen::CheckingKey { .. }));
    settle(&mut sign_in, &receiver);

    assert_eq!(sign_in.connection(), Some(Connection::ApiKey));
}

#[test]
fn going_back_cancels_the_claude_code_sign_in() {
    let connector = FakeConnector::default();
    let cancelled = Arc::clone(&connector.claude_code_cancelled);
    let (mut sign_in, receiver) = sign_in(connector);
    sign_in.start();
    settle(&mut sign_in, &receiver);

    sign_in.input(Input::Press(Connection::ClaudeCode));
    sign_in.input(Input::Back);
    sign_in.stop();

    assert!(cancelled.load(Ordering::Relaxed));
    assert!(receiver.try_recv().is_err());
    assert_eq!(sign_in.screen, Screen::choosing(Connection::ClaudeCode));
}

#[test]
fn finds_a_way_only_on_its_row() {
    let (mut sign_in, receiver) = sign_in(FakeConnector::default());
    sign_in.start();
    settle(&mut sign_in, &receiver);

    assert_eq!(sign_in.method_at(4), Some(Connection::AnthropicAccount));
    assert_eq!(sign_in.method_at(5), Some(Connection::ApiKey));
    assert_eq!(sign_in.method_at(0), None);
}

#[test]
fn signs_in_to_an_anthropic_account_in_the_background() {
    let (mut sign_in, receiver) = sign_in(FakeConnector::default());
    sign_in.start();
    settle(&mut sign_in, &receiver);

    sign_in.input(Input::Press(Connection::AnthropicAccount));
    assert_eq!(sign_in.screen, Screen::WaitingForSignIn(Account::Anthropic));
    settle(&mut sign_in, &receiver);

    assert_eq!(sign_in.connection(), Some(Connection::AnthropicAccount));
}

fn connected(connection: Connection) -> (SignIn<FakeConnector, Sender<Report>>, Receiver<Report>) {
    let connector = FakeConnector {
        connected: Some(connection),
        ..FakeConnector::default()
    };
    let (mut sign_in, receiver) = sign_in(connector);
    sign_in.start();
    settle(&mut sign_in, &receiver);
    (sign_in, receiver)
}

#[test]
fn changing_offers_the_choices_on_the_connection_in_use_and_back_returns_to_it() {
    let (mut sign_in, _receiver) = connected(Connection::AnthropicAccount);

    sign_in.change();
    assert_eq!(sign_in.connection(), None);
    assert_eq!(
        sign_in.screen,
        Screen::choosing(Connection::AnthropicAccount)
    );
    sign_in.input(Input::Back);

    assert_eq!(sign_in.connection(), Some(Connection::AnthropicAccount));
}

#[test]
fn changing_does_nothing_before_a_connection() {
    let (mut sign_in, _receiver) = sign_in(FakeConnector::default());

    sign_in.change();

    assert_eq!(sign_in.screen, Screen::Checking);
}

#[test]
fn back_on_the_choices_does_nothing_without_a_connection_to_return_to() {
    let (mut sign_in, receiver) = sign_in(FakeConnector::default());
    sign_in.start();
    settle(&mut sign_in, &receiver);

    sign_in.input(Input::Back);

    assert_eq!(sign_in.connection(), None);
}

#[test]
fn a_new_connection_forgets_the_one_it_replaced() {
    let (mut sign_in, receiver) = connected(Connection::ClaudeCode);
    sign_in.change();

    sign_in.input(Input::Press(Connection::AnthropicAccount));
    settle(&mut sign_in, &receiver);
    sign_in.change();
    sign_in.input(Input::Back);

    assert_eq!(sign_in.connection(), Some(Connection::AnthropicAccount));
}

#[test]
fn replacing_the_key_connects_with_it_once_anthropic_accepts_it() {
    let (mut sign_in, receiver) = connected(Connection::ClaudeCode);

    sign_in.replace_key(ApiKey::parse(KEY).unwrap());
    let report = receiver.recv_timeout(PATIENCE).unwrap();
    assert!(matches!(report, Report::KeyReplaced(Ok(()))));
    sign_in.report(report);

    assert_eq!(sign_in.connection(), Some(Connection::ApiKey));
}

#[test]
fn a_refused_key_leaves_the_connection_as_it_was() {
    let (mut sign_in, receiver) = connected(Connection::ClaudeCode);

    sign_in.replace_key(ApiKey::parse("sk-ant-api03-refused-key-0000").unwrap());
    settle(&mut sign_in, &receiver);

    assert_eq!(sign_in.connection(), Some(Connection::ClaudeCode));
}

#[test]
fn signing_out_forgets_the_connection_and_offers_the_choices() {
    let connector = FakeConnector {
        connected: Some(Connection::ApiKey),
        ..FakeConnector::default()
    };
    let forgotten = Arc::clone(&connector.forgotten);
    let (mut sign_in, receiver) = sign_in(connector);
    sign_in.start();
    settle(&mut sign_in, &receiver);

    sign_in.sign_out();
    settle(&mut sign_in, &receiver);

    assert!(forgotten.load(Ordering::Relaxed));
    assert_eq!(sign_in.connection(), None);
    assert_eq!(sign_in.screen, Screen::choosing(Connection::ClaudeCode));
}

#[test]
fn a_sign_out_that_fails_keeps_the_connection() {
    let connector = FakeConnector {
        connected: Some(Connection::ApiKey),
        refuses_to_forget: true,
        ..FakeConnector::default()
    };
    let (mut sign_in, receiver) = sign_in(connector);
    sign_in.start();
    settle(&mut sign_in, &receiver);

    sign_in.sign_out();
    let report = receiver.recv_timeout(PATIENCE).unwrap();
    assert!(report.error().is_some());
    sign_in.report(report);

    assert_eq!(sign_in.connection(), Some(Connection::ApiKey));
}
