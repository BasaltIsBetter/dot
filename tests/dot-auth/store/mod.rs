use std::sync::Mutex;

use dot_api::{ApiKey, ConnectFailure};

use super::*;
use crate::store::choice::tests::{ScratchFolder, choice_in};

const KEY: &str = "sk-ant-api03-0123456789abcdef";

#[derive(Default)]
struct FakeKeys {
    stored: Mutex<Option<Remembered>>,
    reads: Mutex<u32>,
}

impl KeyStore for FakeKeys {
    fn load(&self) -> Result<Option<Remembered>, ConnectError> {
        *self.reads.lock().unwrap() += 1;
        Ok(self.stored.lock().unwrap().clone())
    }

    fn save(&self, remembered: &Remembered) -> Result<(), ConnectError> {
        *self.stored.lock().unwrap() = Some(remembered.clone());
        Ok(())
    }

    fn forget(&self) -> Result<(), ConnectError> {
        *self.stored.lock().unwrap() = None;
        Ok(())
    }
}

fn key() -> Remembered {
    Remembered::ApiKey(ApiKey::parse(KEY).unwrap())
}

fn store_in(folder: &ScratchFolder, keys: Option<Remembered>) -> SplitStore<FakeKeys> {
    SplitStore::new(
        Some(choice_in(folder)),
        FakeKeys {
            stored: Mutex::new(keys),
            reads: Mutex::new(0),
        },
    )
}

fn keychain_reads(store: &SplitStore<FakeKeys>) -> u32 {
    *store.keys.reads.lock().unwrap()
}

#[test]
fn keeps_the_claude_code_choice_out_of_the_keychain() {
    let folder = ScratchFolder::new("split-claude");
    let store = store_in(&folder, Some(key()));

    store.save(&Remembered::ClaudeCode).unwrap();

    assert_eq!(store.load().unwrap(), Some(Remembered::ClaudeCode));
    assert_eq!(*store.keys.stored.lock().unwrap(), None);
    assert_eq!(keychain_reads(&store), 0);
}

#[test]
fn keeps_an_api_key_in_the_keychain_only() {
    let folder = ScratchFolder::new("split-key");
    let store = store_in(&folder, None);
    store.save(&Remembered::ClaudeCode).unwrap();

    store.save(&key()).unwrap();

    assert_eq!(store.load().unwrap(), Some(key()));
    assert_eq!(choice_in(&folder).choice().unwrap(), None);
}

#[test]
fn moves_a_claude_code_choice_out_of_the_keychain_once() {
    let folder = ScratchFolder::new("split-migrate");
    let store = store_in(&folder, Some(Remembered::ClaudeCode));

    assert_eq!(store.load().unwrap(), Some(Remembered::ClaudeCode));
    assert_eq!(store.load().unwrap(), Some(Remembered::ClaudeCode));

    assert_eq!(keychain_reads(&store), 1);
    assert_eq!(*store.keys.stored.lock().unwrap(), None);
    assert_eq!(
        choice_in(&folder).choice().unwrap(),
        Some(Remembered::ClaudeCode)
    );
}

#[test]
fn remembers_nothing_when_neither_holds_anything() {
    let folder = ScratchFolder::new("split-nothing");

    assert_eq!(store_in(&folder, None).load().unwrap(), None);
}

#[test]
fn falls_back_on_the_keychain_without_a_home_folder() {
    let store = SplitStore::new(None, FakeKeys::default());

    store.save(&Remembered::ClaudeCode).unwrap();

    assert_eq!(store.load().unwrap(), Some(Remembered::ClaudeCode));
    assert_eq!(
        *store.keys.stored.lock().unwrap(),
        Some(Remembered::ClaudeCode)
    );
}

#[test]
fn reports_a_choice_file_it_cannot_read() {
    let folder = ScratchFolder::new("split-unreadable");
    std::fs::create_dir_all(folder.path().join("Dot/connection")).unwrap();

    let error = store_in(&folder, None).load().unwrap_err();

    assert_eq!(error.failure(), ConnectFailure::StorageFailed);
}

#[test]
fn keeps_the_account_choice_out_of_the_keychain() {
    let folder = ScratchFolder::new("split-account");
    let store = store_in(&folder, Some(key()));

    store.save(&Remembered::AnthropicAccount).unwrap();

    assert_eq!(store.load().unwrap(), Some(Remembered::AnthropicAccount));
    assert_eq!(*store.keys.stored.lock().unwrap(), None);
    assert_eq!(keychain_reads(&store), 0);
}

#[test]
fn names_only_the_choices_without_secret() {
    assert_eq!(
        secretless_name(&Remembered::ClaudeCode),
        Some("claude-code")
    );
    assert_eq!(
        secretless_name(&Remembered::AnthropicAccount),
        Some("anthropic-account")
    );
    assert_eq!(secretless_name(&key()), None);
    assert_eq!(
        secretless_named("anthropic-account"),
        Some(Remembered::AnthropicAccount)
    );
    assert_eq!(
        secretless_named("claude-code"),
        Some(Remembered::ClaudeCode)
    );
    assert_eq!(secretless_named("api-key"), None);
}

#[test]
fn forgets_the_key_and_the_choice_file() {
    let folder = ScratchFolder::new("forget-both");
    let store = store_in(&folder, Some(key()));
    store.save(&Remembered::AnthropicAccount).unwrap();

    store.forget().unwrap();

    assert_eq!(store.load().unwrap(), None);
    assert_eq!(*store.keys.stored.lock().unwrap(), None);
    assert_eq!(choice_in(&folder).choice().unwrap(), None);
}

#[test]
fn forgets_the_keychain_alone_without_a_home_folder() {
    let keys = FakeKeys::default();
    *keys.stored.lock().unwrap() = Some(key());
    let store = SplitStore::new(None, keys);

    store.forget().unwrap();

    assert_eq!(store.load().unwrap(), None);
}
