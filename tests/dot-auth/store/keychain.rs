use proptest::prelude::*;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

fn key() -> ApiKey {
    ApiKey::parse(KEY).unwrap()
}

#[test]
fn an_entry_never_stored_means_nothing_remembered() {
    let store = KeychainStore {
        service: SERVICE,
        account: "dot-tests-never-stored",
    };

    assert_eq!(store.load().unwrap(), None);
}

#[test]
fn forgetting_an_entry_never_stored_is_no_error() {
    let store = KeychainStore {
        service: SERVICE,
        account: "dot-tests-never-stored",
    };

    assert!(store.forget().is_ok());
}

#[test]
fn stores_the_claude_code_choice_without_secret() {
    assert_eq!(encode(&Remembered::ClaudeCode), "claude-code");
}

#[test]
fn stores_the_key_after_its_kind() {
    assert_eq!(
        encode(&Remembered::ApiKey(key())),
        "api-key:sk-ant-api03-0123456789abcdef"
    );
}

#[test]
fn reads_back_what_it_stored() {
    for remembered in [Remembered::ClaudeCode, Remembered::ApiKey(key())] {
        assert_eq!(decode(encode(&remembered).as_bytes()), Some(remembered));
    }
}

#[test]
fn forgets_an_entry_it_cannot_read() {
    assert_eq!(decode(b"api-key:not-a-key"), None);
    assert_eq!(decode(KEY.as_bytes()), None);
    assert_eq!(decode(b"claude-code "), None);
    assert_eq!(decode(&[0xff, 0xfe]), None);
}

proptest! {
    #[test]
    fn reads_back_any_stored_key(body in "[A-Za-z0-9_-]{13,249}") {
        let remembered = Remembered::ApiKey(ApiKey::parse(&format!("sk-ant-{body}")).unwrap());

        prop_assert_eq!(decode(encode(&remembered).as_bytes()), Some(remembered));
    }

    #[test]
    fn reads_anything_without_panicking(stored in proptest::collection::vec(any::<u8>(), 0..300)) {
        if let Some(Remembered::ApiKey(key)) = decode(&stored) {
            prop_assert!(key.expose().starts_with("sk-ant-"));
        }
    }
}
