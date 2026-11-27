use std::borrow::Cow;
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

struct Plain;

impl Translator for Plain {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }

    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
    }

    fn use_language(&self, _language: Language) {}
}

#[test]
fn a_translator_puts_a_value_after_its_text_unless_it_knows_better() {
    assert_eq!(
        Plain.with(Text::StateReading, "layout.rs"),
        "state-reading layout.rs"
    );
}

#[test]
fn finds_each_text_by_its_id() {
    for text in Text::ALL {
        assert_eq!(Text::from_id(text.id()), Some(*text));
    }
}

#[test]
fn finds_no_text_for_an_unknown_id() {
    assert_eq!(Text::from_id("no-such-text"), None);
    assert_eq!(Text::from_id(""), None);
}

#[test]
fn writes_a_text_as_its_id() {
    let written = serde_json::to_string(&Text::ReplyStopped).expect("a text serializes");

    assert_eq!(written, r#""reply-stopped""#);
    assert_eq!(
        serde_json::from_str::<Text>(&written).expect("it reads back"),
        Text::ReplyStopped
    );
}

#[test]
fn refuses_to_read_a_text_no_one_has() {
    assert!(serde_json::from_str::<Text>(r#""no-such-text""#).is_err());
    assert!(serde_json::from_str::<Text>("3").is_err());
}
