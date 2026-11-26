use std::time::Instant;

use super::*;
use dot_api::Hue;

#[test]
fn starts_with_its_colour_and_without_a_name_of_its_own() {
    let identity = Identity::new(Hue::Green);

    assert_eq!(identity.hue, Hue::Green);
    assert_eq!(identity.name, None);
    assert_eq!(identity.renamed, None);
}

#[test]
fn gives_a_new_session_a_colour_no_other_has() {
    let taken = [
        Hue::Orange,
        Hue::Red,
        Hue::Amber,
        Hue::Green,
        Hue::Cyan,
        Hue::Blue,
    ];

    let picked: Vec<Hue> = (0..16).map(|roll| fresh_hue(&taken, roll)).collect();

    assert!(
        picked
            .iter()
            .all(|hue| matches!(hue, Hue::Violet | Hue::Pink))
    );
    assert!(picked.contains(&Hue::Violet) && picked.contains(&Hue::Pink));
    assert_eq!(fresh_hue(&[], 3), Hue::ALL[3]);
    assert_eq!(fresh_hue(&[Hue::Orange], 0), Hue::Red);
}

#[test]
fn takes_any_colour_once_all_eight_are_taken() {
    let picked: Vec<Hue> = (0..8).map(|roll| fresh_hue(&Hue::ALL, roll)).collect();

    assert_eq!(picked, Hue::ALL);
}

#[test]
fn rolls_a_different_number_each_time_from_its_seed() {
    let mut dice = Dice::new(7);
    let mut again = Dice::new(7);
    let mut zero = Dice::new(0);

    let rolls: Vec<u64> = (0..8).map(|_| dice.roll()).collect();

    assert_eq!(rolls, (0..8).map(|_| again.roll()).collect::<Vec<u64>>());
    let mut distinct = rolls.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), 8);
    assert_ne!(zero.roll(), 0);
    assert_ne!(Dice::new(7).roll(), Dice::new(8).roll());
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
    let mut identity = Identity::new(Hue::Violet);

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
    let mut identity = Identity::new(Hue::Violet);

    identity.rename("   ", "Dot", now);
    identity.rename("Dot", "Dot", now);

    assert_eq!(identity.name, None);
    assert_eq!(identity.renamed, None);
    assert_eq!(identity.name_heat(now), 0);
}
