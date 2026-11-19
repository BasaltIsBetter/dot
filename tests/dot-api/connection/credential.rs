use proptest::prelude::*;

use super::*;

const TOKEN: &str = "sk-ant-oat01-Abc_def-0123456789";

#[test]
fn accepts_a_token_shaped_like_ant_s() {
    assert_eq!(AccessToken::parse(TOKEN).unwrap().expose(), TOKEN);
}

#[test]
fn ignores_the_line_break_ant_prints_after_the_token() {
    let printed = format!("{TOKEN}\n");

    assert_eq!(AccessToken::parse(&printed).unwrap().expose(), TOKEN);
}

#[test]
fn rejects_a_token_too_short_or_too_long() {
    let shortest = "a".repeat(SHORTEST);
    let longest = "a".repeat(LONGEST);

    assert!(AccessToken::parse(&shortest).is_ok());
    assert!(AccessToken::parse(&longest).is_ok());
    assert_eq!(AccessToken::parse(&shortest[1..]), Err(AccessTokenError));
    assert_eq!(
        AccessToken::parse(&format!("{longest}a")),
        Err(AccessTokenError)
    );
    assert_eq!(AccessToken::parse(""), Err(AccessTokenError));
}

#[test]
fn rejects_a_token_with_a_space_or_a_foreign_character() {
    assert!(AccessToken::parse("sk-ant-oat01 0123456789abcdef").is_err());
    assert!(AccessToken::parse("sk-ant-oat01-0123456789abcdé").is_err());
}

#[test]
fn never_shows_the_token_when_debugged() {
    let token = AccessToken::parse(TOKEN).unwrap();

    assert_eq!(format!("{token:?}"), "AccessToken(…)");
    assert!(!format!("{:?}", Credential::AccessToken(token)).contains("0123456789"));
}

#[test]
fn explains_a_malformed_token_in_english() {
    assert_eq!(
        AccessTokenError.to_string(),
        "the text is not shaped like an access token"
    );
}

proptest! {
    #[test]
    fn accepts_every_visible_ascii_token(token in "[!-~]{20,4096}") {
        let parsed = AccessToken::parse(&token).unwrap();
        prop_assert_eq!(parsed.expose(), token.as_str());
    }

    #[test]
    fn keeps_only_well_shaped_tokens(text in any::<String>()) {
        if let Ok(token) = AccessToken::parse(&text) {
            prop_assert!((SHORTEST..=LONGEST).contains(&token.expose().len()));
            prop_assert!(token.expose().bytes().all(|byte| byte.is_ascii_graphic()));
        }
    }
}
