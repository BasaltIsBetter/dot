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
    assert!(!choice_in(&folder).holds_claude_code().unwrap());
}

#[test]
fn moves_a_claude_code_choice_out_of_the_keychain_once() {
    let folder = ScratchFolder::new("split-migrate");
    let store = store_in(&folder, Some(Remembered::ClaudeCode));

    assert_eq!(store.load().unwrap(), Some(Remembered::ClaudeCode));
    assert_eq!(store.load().unwrap(), Some(Remembered::ClaudeCode));

    assert_eq!(keychain_reads(&store), 1);
    assert_eq!(*store.keys.stored.lock().unwrap(), None);
    assert!(choice_in(&folder).holds_claude_code().unwrap());
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
