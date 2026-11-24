use std::time::Instant;

use super::*;

fn with(hue: Hue) -> Identity {
    Identity {
        hue,
        name: None,
        renamed: None,
    }
}

#[test]
fn gives_the_first_session_orange() {
    assert_eq!(Identity::new([], 0).hue, Hue::Orange);
}

#[test]
fn gives_each_new_session_the_first_colour_no_other_has() {
    let taken = [with(Hue::Orange), with(Hue::Amber)];

    assert_eq!(Identity::new(&taken, 2).hue, Hue::Red);
}

#[test]
fn starts_the_colours_again_once_all_are_taken() {
    let taken: Vec<Identity> = Hue::ALL.into_iter().map(with).collect();

    assert_eq!(Identity::new(&taken, 8).hue, Hue::Orange);
    assert_eq!(Identity::new(&taken, 10).hue, Hue::Amber);
}

#[test]
fn starts_without_a_name_of_its_own() {
    let identity = Identity::new([], 0);

    assert_eq!(identity.name, None);
    assert_eq!(identity.renamed, None);
}

fn field(shown: &str) -> NameField {
    NameField::of(SessionKey(1), shown)
}

#[test]
fn opens_the_field_with_the_whole_name_chosen() {
    let field = field("Dot");

    assert_eq!(field.text, "Dot");
    assert_eq!(field.caret, 3);
    assert!(field.whole);
}

#[test]
fn replaces_the_whole_name_with_what_is_typed_first() {
    let mut field = field("Dot");

    field.type_text("api");

    assert_eq!(field.text, "api");
    assert_eq!(field.caret, 3);
    assert!(!field.whole);
}

#[test]
fn keeps_at_most_twenty_four_characters() {
    let mut field = field(&"x".repeat(30));
    assert_eq!(field.text.chars().count(), 24);

    field.type_text(&"é".repeat(30));

    assert_eq!(field.text, "é".repeat(24));
    field.type_text("z");
    assert_eq!(field.text.chars().count(), 24);
}

#[test]
fn types_where_the_caret_stands_and_skips_controls() {
    let mut field = field("ac");
    field.right();
    field.left();

    field.type_text("b\n");

    assert_eq!(field.text, "abc");
    assert_eq!(field.caret, 2);
}

#[test]
fn erases_the_whole_name_then_one_character_before_the_caret() {
    let mut field = field("Dot");

    field.erase();
    assert_eq!(field.text, "");
    field.type_text("éa");
    field.erase();
    assert_eq!(field.text, "é");
    field.left();
    field.erase();
    assert_eq!(field.text, "é");
    assert_eq!(field.caret, 0);
}

#[test]
fn moves_the_caret_within_the_name() {
    let mut field = field("ab");

    field.left();
    assert_eq!(field.caret, 0);
    assert!(!field.whole);
    field.left();
    assert_eq!(field.caret, 0);
    field.right();
    field.right();
    field.right();
    assert_eq!(field.caret, 2);
}

#[test]
fn right_keeps_the_caret_at_the_end_of_a_whole_name() {
    let mut field = field("ab");

    field.right();

    assert_eq!(field.caret, 2);
    assert!(!field.whole);
}

#[test]
fn takes_a_new_name_and_warms_it_for_a_second_and_a_fifth() {
    let now = Instant::now();
    let mut identity = with(Hue::Orange);

    identity.rename("  api  ", "Dot", now);

    assert_eq!(identity.name.as_deref(), Some("api"));
    assert_eq!(identity.name_heat(now), u8::MAX);
    assert!(identity.is_cooling(now + std::time::Duration::from_millis(1_100)));
    assert_eq!(
        identity.name_heat(now + std::time::Duration::from_millis(1_200)),
        0
    );
    assert!(!identity.is_cooling(now + std::time::Duration::from_millis(1_200)));
}

#[test]
fn keeps_the_old_name_when_the_new_one_is_empty_or_the_same() {
    let now = Instant::now();
    let mut identity = with(Hue::Orange);

    identity.rename("   ", "Dot", now);
    identity.rename("Dot", "Dot", now);

    assert_eq!(identity.name, None);
    assert_eq!(identity.renamed, None);
    assert_eq!(identity.name_heat(now), 0);
}
