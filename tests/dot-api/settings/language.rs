use super::*;

#[test]
fn codes_each_language_like_iso_639_1() {
    assert_eq!(Language::French.code(), "fr");
    assert_eq!(Language::English.code(), "en");
}

#[test]
fn finds_a_language_by_its_code_whatever_its_case() {
    assert_eq!(Language::with_code("fr"), Some(Language::French));
    assert_eq!(Language::with_code("EN"), Some(Language::English));
}

#[test]
fn refuses_a_code_dot_does_not_speak() {
    assert_eq!(Language::with_code("de"), None);
    assert_eq!(Language::with_code("fra"), None);
    assert_eq!(Language::with_code(""), None);
}

#[test]
fn names_each_language_in_itself() {
    assert_eq!(Language::French.own_name(), "français");
    assert_eq!(Language::English.own_name(), "english");
}

#[test]
fn lists_every_language_once() {
    assert_eq!(Language::ALL, [Language::French, Language::English]);
}
