use proptest::prelude::*;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

#[test]
fn hands_the_text_back_only_when_exposed() {
    let secret = Secret::new(KEY);

    assert_eq!(secret.expose(), KEY);
    assert_eq!(Secret::default().expose(), "");
}

#[test]
fn never_shows_the_text_when_debugged() {
    let secret = Secret::new(KEY);

    assert_eq!(format!("{secret:?}"), "…");
    assert_eq!(format!("{:?}", Some(&secret)), "Some(…)");
    assert_eq!(format!("{:#?}", [secret]), "[\n    …,\n]");
}

#[test]
fn changes_in_place_and_stays_masked() {
    let mut secret = Secret::new("sk-ant");

    secret.expose_mut().push_str("-api03");
    secret.expose_mut().pop();

    assert_eq!(secret.expose(), "sk-ant-api0");
    assert_eq!(format!("{secret:?}"), "…");
}

#[test]
fn two_secrets_are_equal_when_their_texts_are() {
    assert_eq!(Secret::new(KEY), Secret::new(KEY.to_owned()));
    assert_ne!(Secret::new(KEY), Secret::default());
}

proptest! {
    #[test]
    fn debug_never_carries_any_of_the_text(text in "[!-~]{1,64}") {
        let debugged = format!("{:?}", Secret::new(text));

        prop_assert_eq!(debugged, "…");
    }
}

#[test]
fn keeps_what_is_typed_in_the_room_it_reserved() {
    let mut secret = Secret::with_capacity(256);
    let before = secret.expose_mut().as_ptr();

    for _ in 0..256 {
        secret.expose_mut().push('k');
    }

    assert_eq!(secret.expose_mut().as_ptr(), before);
    assert_eq!(secret.expose().len(), 256);
    assert_eq!(format!("{secret:?}"), "…");
}
