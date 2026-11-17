use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use dot_api::{ApiKey, ConnectError, ConnectFailure, Connection};

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);
const KEY: &str = "sk-ant-api03-0123456789abcdef";

#[derive(Default)]
pub(crate) struct FakeConnector {
    pub(crate) connected: Option<Connection>,
    pub(crate) claude_code_cancelled: Arc<AtomicBool>,
}

impl Connector for FakeConnector {
    fn current(&self) -> Result<Option<Connection>, ConnectError> {
        Ok(self.connected)
    }

    fn connect_with_api_key(&self, key: &ApiKey) -> Result<(), ConnectError> {
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
}

fn sign_in(connector: FakeConnector) -> (SignIn<FakeConnector, Sender<Report>>, Receiver<Report>) {
    let (sender, receiver) = mpsc::channel();
    (SignIn::new(connector, sender), receiver)
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

    sign_in.input(Input::Press(Method::ApiKey));
    sign_in.input(Input::Paste(KEY.to_owned()));
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

    sign_in.input(Input::Press(Method::ClaudeCode));
    sign_in.input(Input::Back);
    sign_in.stop();

    assert!(cancelled.load(Ordering::Relaxed));
    assert!(receiver.try_recv().is_err());
    assert_eq!(sign_in.screen, Screen::choosing(Method::ClaudeCode));
}

#[test]
fn finds_a_way_only_on_its_row() {
    let (mut sign_in, receiver) = sign_in(FakeConnector::default());
    sign_in.start();
    settle(&mut sign_in, &receiver);

    assert_eq!(sign_in.method_at(4), Some(Method::ApiKey));
    assert_eq!(sign_in.method_at(0), None);
}
