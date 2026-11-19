use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use dot_api::AccessToken;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";
const TOKEN: &str = "sk-ant-oat01-0123456789abcdef";

#[derive(Default)]
struct FakeStore {
    remembered: Mutex<Option<Remembered>>,
    broken: bool,
}

struct FakeVerifier(fn() -> Verdict);

#[derive(Default)]
struct FakeAnt {
    signed_in: AtomicBool,
    sign_in_works: bool,
    sign_in_count: AtomicU32,
    token_count: AtomicU32,
}

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

impl AnthropicAccount for FakeAnt {
    fn access_token(&self) -> Result<AccessToken, ConnectError> {
        self.token_count.fetch_add(1, Ordering::Relaxed);
        if !self.signed_in.load(Ordering::Relaxed) {
            return Err(ConnectError::new(ConnectFailure::AccountFailed));
        }
        Ok(AccessToken::parse(TOKEN).unwrap())
    }

    fn sign_in(&self, _cancellation: &Cancellation) -> Result<(), ConnectError> {
        self.sign_in_count.fetch_add(1, Ordering::Relaxed);
        if !self.sign_in_works {
            return Err(ConnectError::new(ConnectFailure::AccountFailed));
        }
        self.signed_in.store(true, Ordering::Relaxed);
        Ok(())
    }
}

type TestAccounts = Accounts<FakeStore, FakeVerifier, FakeClaudeCode, FakeAnt>;

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
    with_ant(remembered, claude_code, None)
}

fn with_ant(
    remembered: Option<Remembered>,
    claude_code: Option<FakeClaudeCode>,
    ant: Option<FakeAnt>,
) -> TestAccounts {
    Accounts::new(
        FakeStore {
            remembered: Mutex::new(remembered),
            broken: false,
        },
        FakeVerifier(accepted),
        claude_code,
        ant,
    )
}

fn signed_in_ant() -> FakeAnt {
    FakeAnt {
        signed_in: AtomicBool::new(true),
        ..FakeAnt::default()
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

    let accounts: Accounts<_, _, _, FakeAnt> = Accounts::new(
        FakeStore::default(),
        FakeVerifier(accepted),
        Some(SignedOutClaudeCode),
        None,
    );

    let failure = failure_of(accounts.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::ClaudeCodeFailed);
    assert_eq!(*accounts.store.remembered.lock().unwrap(), None);
}

#[test]
fn a_remembered_account_counts_while_ant_hands_out_a_token() {
    let signed_in = with_ant(
        Some(Remembered::AnthropicAccount),
        None,
        Some(signed_in_ant()),
    );
    let signed_out = with_ant(
        Some(Remembered::AnthropicAccount),
        None,
        Some(FakeAnt::default()),
    );
    let uninstalled = with_ant(Some(Remembered::AnthropicAccount), None, None);

    assert_eq!(
        signed_in.current().unwrap(),
        Some(Connection::AnthropicAccount)
    );
    assert_eq!(signed_out.current().unwrap(), None);
    assert_eq!(uninstalled.current().unwrap(), None);
}

#[test]
fn asks_for_ant_when_it_is_missing() {
    let accounts = accounts(None, None);

    let failure = failure_of(accounts.connect_with_anthropic_account(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::AntMissing);
    assert_eq!(remembered(&accounts), None);
}

#[test]
fn reuses_an_account_ant_is_already_signed_in_to() {
    let accounts = with_ant(None, None, Some(signed_in_ant()));

    accounts
        .connect_with_anthropic_account(&Cancellation::new())
        .unwrap();

    let ant = accounts.ant.as_ref().unwrap();
    assert_eq!(ant.sign_in_count.load(Ordering::Relaxed), 0);
    assert_eq!(remembered(&accounts), Some(Remembered::AnthropicAccount));
}

#[test]
fn signs_in_through_ant_then_remembers_the_account() {
    let ant = FakeAnt {
        sign_in_works: true,
        ..FakeAnt::default()
    };
    let accounts = with_ant(Some(Remembered::ApiKey(key())), None, Some(ant));

    accounts
        .connect_with_anthropic_account(&Cancellation::new())
        .unwrap();

    let ant = accounts.ant.as_ref().unwrap();
    assert_eq!(ant.sign_in_count.load(Ordering::Relaxed), 1);
    assert_eq!(remembered(&accounts), Some(Remembered::AnthropicAccount));
}

#[test]
fn forgets_nothing_when_the_ant_sign_in_fails() {
    let accounts = with_ant(None, None, Some(FakeAnt::default()));

    let failure = failure_of(accounts.connect_with_anthropic_account(&Cancellation::new()));

    assert_eq!(failure, ConnectFailure::AccountFailed);
    assert_eq!(remembered(&accounts), None);
}

#[test]
fn hands_out_the_remembered_api_key() {
    let accounts = accounts(Some(Remembered::ApiKey(key())), None);

    assert_eq!(accounts.credential().unwrap(), Credential::ApiKey(key()));
}

#[test]
fn reads_the_api_key_once_then_keeps_it_in_memory() {
    let accounts = accounts(Some(Remembered::ApiKey(key())), None);
    accounts.current().unwrap();
    *accounts.store.remembered.lock().unwrap() = None;

    assert_eq!(accounts.credential().unwrap(), Credential::ApiKey(key()));
}

#[test]
fn reads_the_store_when_nothing_is_kept_yet() {
    let accounts = accounts(Some(Remembered::ApiKey(key())), None);

    assert_eq!(accounts.credential().unwrap(), Credential::ApiKey(key()));
}

#[test]
fn hands_out_the_key_connected_last() {
    let accounts = accounts(Some(Remembered::ClaudeCode), None);
    accounts.current().unwrap();

    accounts.connect_with_api_key(&key()).unwrap();

    assert_eq!(accounts.credential().unwrap(), Credential::ApiKey(key()));
}

#[test]
fn asks_ant_for_a_fresh_token_on_each_request() {
    let accounts = with_ant(
        Some(Remembered::AnthropicAccount),
        None,
        Some(signed_in_ant()),
    );

    let first = accounts.credential().unwrap();
    accounts.credential().unwrap();

    assert_eq!(
        first,
        Credential::AccessToken(AccessToken::parse(TOKEN).unwrap())
    );
    let ant = accounts.ant.as_ref().unwrap();
    assert_eq!(ant.token_count.load(Ordering::Relaxed), 2);
}

#[test]
fn reports_an_account_ant_no_longer_hands_a_token_for() {
    let accounts = with_ant(
        Some(Remembered::AnthropicAccount),
        None,
        Some(FakeAnt::default()),
    );

    let failure = accounts.credential().unwrap_err().failure();

    assert_eq!(failure, ConnectFailure::AccountFailed);
}

#[test]
fn lets_claude_code_carry_its_own_credential() {
    let accounts = accounts(Some(Remembered::ClaudeCode), None);

    assert_eq!(accounts.credential().unwrap(), Credential::ClaudeCode);
}

#[test]
fn has_no_credential_when_nothing_is_connected() {
    let accounts = accounts(None, None);

    let failure = accounts.credential().unwrap_err().failure();

    assert_eq!(failure, ConnectFailure::NotConnected);
}
