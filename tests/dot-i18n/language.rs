use proptest::prelude::*;

use super::*;

const UNSUPPORTED_TAG: &str = "[a-dg-z][a-z]{1,2}([-_][A-Za-z0-9]{2,8}){0,2}";

#[test]
fn picks_french_for_a_french_mac() {
    assert_eq!(Language::negotiate(["fr-FR"]), Language::French);
}

#[test]
fn picks_english_for_an_english_mac() {
    assert_eq!(Language::negotiate(["en-US"]), Language::English);
}

#[test]
fn reads_tags_written_with_an_underscore() {
    assert_eq!(Language::negotiate(["en_GB"]), Language::English);
}

#[test]
fn ignores_the_case_of_the_tag() {
    assert_eq!(Language::negotiate(["EN"]), Language::English);
}

#[test]
fn keeps_the_order_of_the_preferences() {
    assert_eq!(
        Language::negotiate(["de-DE", "en-US", "fr-FR"]),
        Language::English
    );
}

#[test]
fn does_not_mistake_a_longer_language_code() {
    assert_eq!(Language::negotiate(["frr-DE", "en-US"]), Language::English);
}

#[test]
fn falls_back_to_french_without_a_supported_language() {
    assert_eq!(Language::negotiate(["de-DE", "ja-JP"]), Language::French);
}

#[test]
fn falls_back_to_french_without_any_preference() {
    assert_eq!(Language::negotiate(Vec::<String>::new()), Language::French);
}

#[test]
fn every_language_has_its_own_file() {
    assert_ne!(Language::French.texts(), Language::English.texts());
    assert!(Language::French.texts().contains("menu-quit = Quitter"));
    assert!(Language::English.texts().contains("menu-quit = Quit"));
}

proptest! {
    #[test]
    fn never_panics_on_any_preference(tags in prop::collection::vec(any::<String>(), 0..8)) {
        let language = Language::negotiate(&tags);

        prop_assert!(Language::ALL.contains(&language));
    }

    #[test]
    fn takes_the_first_supported_language_after_unsupported_ones(
        unsupported in prop::collection::vec(UNSUPPORTED_TAG, 0..5),
        region in "[A-Z]{2}",
    ) {
        let mut tags = unsupported;
        tags.push(format!("en-{region}"));
        tags.push(format!("fr-{region}"));

        prop_assert_eq!(Language::negotiate(&tags), Language::English);
    }

    #[test]
    fn falls_back_to_french_when_nothing_is_supported(
        unsupported in prop::collection::vec(UNSUPPORTED_TAG, 0..5),
    ) {
        prop_assert_eq!(Language::negotiate(&unsupported), Language::French);
    }
}
