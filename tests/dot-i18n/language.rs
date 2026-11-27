use proptest::prelude::*;

use super::*;

const UNSUPPORTED_TAG: &str = "[a-dg-oq-z][a-z]{1,2}([-_][A-Za-z0-9]{2,8}){0,2}";

#[test]
fn picks_french_for_a_french_mac() {
    assert_eq!(negotiate(["fr-FR"]), Language::French);
}

#[test]
fn picks_english_for_an_english_mac() {
    assert_eq!(negotiate(["en-US"]), Language::English);
}

#[test]
fn picks_spanish_for_any_spanish_region() {
    assert_eq!(negotiate(["es-ES"]), Language::Spanish);
    assert_eq!(negotiate(["es-419"]), Language::Spanish);
}

#[test]
fn picks_portuguese_for_brazil_and_portugal() {
    assert_eq!(negotiate(["pt-BR"]), Language::Portuguese);
    assert_eq!(negotiate(["pt-PT"]), Language::Portuguese);
}

#[test]
fn reads_tags_written_with_an_underscore() {
    assert_eq!(negotiate(["en_GB"]), Language::English);
}

#[test]
fn ignores_the_case_of_the_tag() {
    assert_eq!(negotiate(["EN"]), Language::English);
}

#[test]
fn keeps_the_order_of_the_preferences() {
    assert_eq!(negotiate(["de-DE", "en-US", "fr-FR"]), Language::English);
}

#[test]
fn does_not_mistake_a_longer_language_code() {
    assert_eq!(negotiate(["frr-DE", "en-US"]), Language::English);
}

#[test]
fn falls_back_to_english_without_a_supported_language() {
    assert_eq!(negotiate(["de-DE", "ja-JP"]), Language::English);
}

#[test]
fn falls_back_to_english_without_any_preference() {
    assert_eq!(negotiate(Vec::<String>::new()), Language::English);
}

#[test]
fn every_language_has_its_own_file() {
    assert!(texts(Language::English).contains("menu-quit = Quit "));
    assert!(texts(Language::French).contains("menu-quit = Quitter "));
    assert!(texts(Language::Spanish).contains("menu-quit = Salir de "));
    assert!(texts(Language::Portuguese).contains("menu-quit = Encerrar "));
}

proptest! {
    #[test]
    fn never_panics_on_any_preference(tags in prop::collection::vec(any::<String>(), 0..8)) {
        let language = negotiate(&tags);

        prop_assert!(Language::ALL.contains(&language));
    }

    #[test]
    fn takes_the_first_supported_language_after_unsupported_ones(
        unsupported in prop::collection::vec(UNSUPPORTED_TAG, 0..5),
        region in "[A-Z]{2}",
    ) {
        let mut tags = unsupported;
        tags.push(format!("es-{region}"));
        tags.push(format!("fr-{region}"));

        prop_assert_eq!(negotiate(&tags), Language::Spanish);
    }

    #[test]
    fn falls_back_to_english_when_nothing_is_supported(
        unsupported in prop::collection::vec(UNSUPPORTED_TAG, 0..5),
    ) {
        prop_assert_eq!(negotiate(&unsupported), Language::English);
    }
}
