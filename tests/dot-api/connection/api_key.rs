use proptest::prelude::*;

use super::*;

const VALID_KEY: &str = "sk-ant-api03-Abc_def-0123456789";

#[test]
fn accepts_a_key_shaped_like_anthropic_s() {
    assert_eq!(ApiKey::parse(VALID_KEY).unwrap().expose(), VALID_KEY);
}

#[test]
fn ignores_the_blank_around_a_pasted_key() {
    let pasted = format!("  {VALID_KEY}\n");

    assert_eq!(ApiKey::parse(&pasted).unwrap().expose(), VALID_KEY);
}

#[test]
fn rejects_a_text_without_the_prefix() {
    assert_eq!(
        ApiKey::parse("sk-proj-0123456789abcdef"),
        Err(ApiKeyError::MissingPrefix)
    );
    assert_eq!(ApiKey::parse(""), Err(ApiKeyError::MissingPrefix));
}

#[test]
fn rejects_a_key_too_short_or_too_long() {
    let longest = format!("{PREFIX}{}", "a".repeat(LONGEST - PREFIX.len()));
    let too_long = format!("{longest}a");
    let shortest = format!("{PREFIX}{}", "a".repeat(SHORTEST - PREFIX.len()));
    let too_short = &shortest[..SHORTEST - 1];

    assert!(ApiKey::parse(&longest).is_ok());
    assert!(ApiKey::parse(&shortest).is_ok());
    assert_eq!(ApiKey::parse(&too_long), Err(ApiKeyError::WrongLength));
    assert_eq!(ApiKey::parse(too_short), Err(ApiKeyError::WrongLength));
}

#[test]
fn rejects_a_key_with_a_foreign_character() {
    assert_eq!(
        ApiKey::parse("sk-ant-api03-abc def-0123"),
        Err(ApiKeyError::InvalidCharacter)
    );
    assert_eq!(
        ApiKey::parse("sk-ant-api03-abcdéf-0123"),
        Err(ApiKeyError::InvalidCharacter)
    );
}

#[test]
fn never_shows_the_key_when_debugged() {
    let debugged = format!("{:?}", ApiKey::parse(VALID_KEY).unwrap());

    assert!(!debugged.contains("api03"));
    assert_eq!(debugged, "ApiKey(sk-ant-…)");
}

proptest! {
    #[test]
    fn accepts_every_well_shaped_key(body in "[A-Za-z0-9_-]{13,249}") {
        let key = format!("{PREFIX}{body}");

        let parsed = ApiKey::parse(&key).unwrap();

        prop_assert_eq!(parsed.expose(), key.as_str());
    }

    #[test]
    fn keeps_only_well_shaped_keys(text in any::<String>()) {
        if let Ok(key) = ApiKey::parse(&text) {
            let exposed = key.expose();
            prop_assert!(exposed.starts_with(PREFIX));
            prop_assert!((SHORTEST..=LONGEST).contains(&exposed.len()));
            prop_assert!(exposed.bytes().all(is_key_byte));
        }
    }
}
