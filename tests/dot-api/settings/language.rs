use super::*;

#[test]
fn codes_each_language_like_iso_639_1() {
    assert_eq!(Language::English.code(), "en");
    assert_eq!(Language::French.code(), "fr");
    assert_eq!(Language::Spanish.code(), "es");
    assert_eq!(Language::Portuguese.code(), "pt");
}

#[test]
fn finds_a_language_by_its_code_whatever_its_case() {
    assert_eq!(Language::with_code("fr"), Some(Language::French));
    assert_eq!(Language::with_code("EN"), Some(Language::English));
    assert_eq!(Language::with_code("Es"), Some(Language::Spanish));
    assert_eq!(Language::with_code("pt"), Some(Language::Portuguese));
}

#[test]
fn refuses_a_code_dot_does_not_speak() {
    assert_eq!(Language::with_code("de"), None);
    assert_eq!(Language::with_code("fra"), None);
    assert_eq!(Language::with_code("pt-BR"), None);
    assert_eq!(Language::with_code(""), None);
}

#[test]
fn names_each_language_in_itself() {
    assert_eq!(Language::English.own_name(), "english");
    assert_eq!(Language::French.own_name(), "français");
    assert_eq!(Language::Spanish.own_name(), "español");
    assert_eq!(Language::Portuguese.own_name(), "português");
}

#[test]
fn lists_every_language_once_english_first() {
    assert_eq!(
        Language::ALL,
        [
            Language::English,
            Language::French,
            Language::Spanish,
            Language::Portuguese
        ]
    );
}
