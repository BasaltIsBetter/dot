use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

#[derive(Default)]
struct FakeStore {
    remembered: Mutex<Option<Remembered>>,
    broken: bool,
}

struct FakeVerifier(fn() -> Verdict);

#[derive(Default)]
struct FakeClaudeCode {
    signed_in: AtomicBool,
    sign_in_works: bool,
    sign_in_count: AtomicU32,
}

impl ConnectionStore for FakeStore {
    fn load(&self) -> Result<Option<Remembered>, ConnectError> {
        if self.broken {
            return Err(ConnectError::new(ConnectFailure::StorageFailed));
        }
        Ok(self.remembered.lock().unwrap().clone())
    }

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError> {
        if self.broken {
            return Err(ConnectError::new(ConnectFailure::StorageFailed));
        }
        *self.remembered.lock().unwrap() = Some(remembered.clone());
        Ok(())
    }
}

impl KeyVerifier for FakeVerifier {
    fn verify(&self, _key: &ApiKey) -> Verdict {
        (self.0)()
    }
}

impl ClaudeCodeAccount for FakeClaudeCode {
    fn is_signed_in(&self) -> Result<bool, ConnectError> {
        Ok(self.signed_in.load(Ordering::Relaxed))
    }

    fn sign_in(&self, _cancellation: &Cancellation) -> Result<(), ConnectError> {
        self.sign_in_count.fetch_add(1, Ordering::Relaxed);
        if !self.sign_in_works {
            return Err(ConnectError::new(ConnectFailure::ClaudeCodeFailed));
        }
        self.signed_in.store(true, Ordering::Relaxed);
        Ok(())
    }
}

type TestAccounts = Accounts<FakeStore, FakeVerifier, FakeClaudeCode>;

fn accepted() -> Verdict {
    Verdict::Accepted
}

fn rejected() -> Verdict {
    Verdict::Rejected
}

fn unreachable() -> Verdict {
    Verdict::Unreachable(Box::new(io::Error::other("timed out")))
}

fn accounts(remembered: Option<Remembered>, claude_code: Option<FakeClaudeCode>) -> TestAccounts {
    Accounts {
        store: FakeStore {
            remembered: Mutex::new(remembered),
            broken: false,
        },
        verifier: FakeVerifier(accepted),
        claude_code,
    }
}

fn key() -> ApiKey {
    ApiKey::parse(KEY).unwrap()
}

fn signed_in_claude_code() -> FakeClaudeCode {
    FakeClaudeCode {
        signed_in: AtomicBool::new(true),
        ..FakeClaudeCode::default()
    }
}

fn remembered(accounts: &TestAccounts) -> Option<Remembered> {
    accounts.store.remembered.lock().unwrap().clone()
}

fn failure_of(result: Result<(), ConnectError>) -> ConnectFailure {
    result.unwrap_err().failure()
}

#[test]
fn the_real_connector_asks_for_claude_code_when_it_is_missing() {
    let connector = AnthropicConnector::new(None, None);

    let failure = failure_of(connector.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::ClaudeCodeMissing);
}

#[test]
fn the_real_connector_reports_a_claude_code_that_cannot_sign_in() {
    let connector = AnthropicConnector::new(Some(PathBuf::from("/usr/bin/false")), None);

    let failure = failure_of(connector.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::ClaudeCodeFailed);
}

#[test]
fn nothing_remembered_means_not_connected() {
    let accounts = accounts(None, Some(signed_in_claude_code()));

    assert_eq!(accounts.current().unwrap(), None);
}

#[test]
fn a_remembered_key_means_connected_with_it() {
    let accounts = accounts(Some(Remembered::ApiKey(key())), None);

    assert_eq!(accounts.current().unwrap(), Some(Connection::ApiKey));
}

#[test]
fn a_remembered_claude_code_counts_while_it_is_signed_in() {
    let accounts = accounts(Some(Remembered::ClaudeCode), Some(signed_in_claude_code()));

    assert_eq!(accounts.current().unwrap(), Some(Connection::ClaudeCode));
}

#[test]
fn a_remembered_claude_code_no_longer_counts_once_signed_out() {
    let accounts = accounts(
        Some(Remembered::ClaudeCode),
        Some(FakeClaudeCode::default()),
    );

    assert_eq!(accounts.current().unwrap(), None);
}

#[test]
fn a_remembered_claude_code_no_longer_counts_once_uninstalled() {
    let accounts = accounts(Some(Remembered::ClaudeCode), None);

    assert_eq!(accounts.current().unwrap(), None);
}

#[test]
fn reports_a_keychain_that_cannot_be_read() {
    let mut accounts = accounts(None, None);
    accounts.store.broken = true;

    let failure = accounts.current().unwrap_err().failure();

    assert_eq!(failure, ConnectFailure::StorageFailed);
}

#[test]
fn remembers_a_key_that_anthropic_accepts() {
    let accounts = accounts(Some(Remembered::ClaudeCode), None);

    accounts.connect_with_api_key(&key()).unwrap();

    assert_eq!(remembered(&accounts), Some(Remembered::ApiKey(key())));
}

#[test]
fn forgets_nothing_when_anthropic_rejects_the_key() {
    let mut accounts = accounts(None, None);
    accounts.verifier = FakeVerifier(rejected);

    let failure = failure_of(accounts.connect_with_api_key(&key()));

    assert_eq!(failure, ConnectFailure::KeyRejected);
    assert_eq!(remembered(&accounts), None);
}

#[test]
fn keeps_the_cause_when_anthropic_cannot_be_reached() {
    let mut accounts = accounts(None, None);
    accounts.verifier = FakeVerifier(unreachable);

    let error = accounts.connect_with_api_key(&key()).unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::AnthropicUnreachable);
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(remembered(&accounts), None);
}

#[test]
fn reports_a_key_that_cannot_be_stored() {
    let mut accounts = accounts(None, None);
    accounts.store.broken = true;

    let failure = failure_of(accounts.connect_with_api_key(&key()));

    assert_eq!(failure, ConnectFailure::StorageFailed);
}

#[test]
fn asks_for_claude_code_when_it_is_missing() {
    let accounts = accounts(None, None);

    let failure = failure_of(accounts.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::ClaudeCodeMissing);
}

#[test]
fn reuses_a_claude_code_already_signed_in() {
    let accounts = accounts(None, Some(signed_in_claude_code()));

    accounts
        .connect_with_claude_code(&Cancellation::new())
        .unwrap();

    let claude_code = accounts.claude_code.as_ref().unwrap();
    assert_eq!(claude_code.sign_in_count.load(Ordering::Relaxed), 0);
    assert_eq!(remembered(&accounts), Some(Remembered::ClaudeCode));
}

#[test]
fn signs_in_through_claude_code_then_remembers_it() {
    let claude_code = FakeClaudeCode {
        sign_in_works: true,
        ..FakeClaudeCode::default()
    };
    let accounts = accounts(Some(Remembered::ApiKey(key())), Some(claude_code));

    accounts
        .connect_with_claude_code(&Cancellation::new())
        .unwrap();

    let claude_code = accounts.claude_code.as_ref().unwrap();
    assert_eq!(claude_code.sign_in_count.load(Ordering::Relaxed), 1);
    assert_eq!(remembered(&accounts), Some(Remembered::ClaudeCode));
}

#[test]
fn forgets_nothing_when_the_claude_code_sign_in_fails() {
    let accounts = accounts(None, Some(FakeClaudeCode::default()));

    let failure = failure_of(accounts.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::ClaudeCodeFailed);
    assert_eq!(remembered(&accounts), None);
}

#[test]
fn fails_when_claude_code_is_still_signed_out_after_its_sign_in() {
    struct SignedOutClaudeCode;

    impl ClaudeCodeAccount for SignedOutClaudeCode {
        fn is_signed_in(&self) -> Result<bool, ConnectError> {
            Ok(false)
        }

        fn sign_in(&self, _cancellation: &Cancellation) -> Result<(), ConnectError> {
            Ok(())
        }
    }

    let accounts = Accounts {
        store: FakeStore::default(),
        verifier: FakeVerifier(accepted),
        claude_code: Some(SignedOutClaudeCode),
    };

    let failure = failure_of(accounts.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::ClaudeCodeFailed);
    assert_eq!(*accounts.store.remembered.lock().unwrap(), None);
}
