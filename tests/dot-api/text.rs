use std::collections::BTreeSet;

use super::*;

fn is_fluent_identifier(id: &str) -> bool {
    let mut characters = id.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

#[test]
fn every_text_has_its_own_id() {
    let ids: BTreeSet<_> = Text::ALL.iter().map(|text| text.id()).collect();

    assert_eq!(ids.len(), Text::ALL.len());
}

#[test]
fn every_id_is_a_fluent_identifier() {
    for text in Text::ALL {
        assert!(is_fluent_identifier(text.id()), "{}", text.id());
    }
}

#[test]
fn recognises_a_fluent_identifier() {
    assert!(is_fluent_identifier("menu-quit"));
    assert!(!is_fluent_identifier("Menu quit"));
    assert!(!is_fluent_identifier("-menu"));
    assert!(!is_fluent_identifier(""));
}

#[test]
fn names_the_quit_item_after_its_role() {
    assert_eq!(Text::MenuQuit.id(), "menu-quit");
}
