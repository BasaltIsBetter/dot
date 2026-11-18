use winit::keyboard::SmolStr;

use super::*;

const NONE: ModifiersState = ModifiersState::empty();
const SHIFT: ModifiersState = ModifiersState::SHIFT;
const ALT: ModifiersState = ModifiersState::ALT;
const COMMAND: ModifiersState = ModifiersState::SUPER;
const CONTROL: ModifiersState = ModifiersState::CONTROL;

fn named(key: NamedKey, modifiers: ModifiersState) -> Option<Intent> {
    intent(&Key::Named(key), None, modifiers)
}

fn character(text: &str, modifiers: ModifiersState) -> Option<Intent> {
    intent(&Key::Character(SmolStr::new(text)), Some(text), modifiers)
}

fn moves(motion: Motion) -> Intent {
    Intent::Edit(Edit::Move(motion))
}

fn erases(motion: Motion) -> Intent {
    Intent::Edit(Edit::Erase(motion))
}

fn inserts(text: &str) -> Intent {
    Intent::Edit(Edit::Insert(text.to_owned()))
}

#[test]
fn return_sends_and_shift_or_option_return_adds_a_line() {
    assert_eq!(named(NamedKey::Enter, NONE), Some(Intent::Send));
    assert_eq!(named(NamedKey::Enter, SHIFT), Some(inserts("\n")));
    assert_eq!(named(NamedKey::Enter, ALT), Some(inserts("\n")));
}

#[test]
fn types_characters_including_option_symbols() {
    assert_eq!(character("é", NONE), Some(inserts("é")));
    assert_eq!(character("A", SHIFT), Some(inserts("A")));
    assert_eq!(character("{", ALT), Some(inserts("{")));
    assert_eq!(
        intent(&Key::Named(NamedKey::Space), Some(" "), NONE),
        Some(inserts(" "))
    );
}

#[test]
fn arrows_move_by_character_word_row_and_text() {
    assert_eq!(named(NamedKey::ArrowLeft, NONE), Some(moves(Motion::Left)));
    assert_eq!(
        named(NamedKey::ArrowRight, NONE),
        Some(moves(Motion::Right))
    );
    assert_eq!(named(NamedKey::ArrowUp, NONE), Some(moves(Motion::Up)));
    assert_eq!(named(NamedKey::ArrowDown, NONE), Some(moves(Motion::Down)));
    assert_eq!(
        named(NamedKey::ArrowLeft, ALT),
        Some(moves(Motion::WordLeft))
    );
    assert_eq!(
        named(NamedKey::ArrowRight, ALT),
        Some(moves(Motion::WordRight))
    );
    assert_eq!(
        named(NamedKey::ArrowLeft, COMMAND),
        Some(moves(Motion::RowStart))
    );
    assert_eq!(
        named(NamedKey::ArrowRight, COMMAND),
        Some(moves(Motion::RowEnd))
    );
    assert_eq!(
        named(NamedKey::ArrowUp, COMMAND),
        Some(moves(Motion::Start))
    );
    assert_eq!(
        named(NamedKey::ArrowDown, COMMAND),
        Some(moves(Motion::End))
    );
    assert_eq!(named(NamedKey::Home, NONE), Some(moves(Motion::RowStart)));
    assert_eq!(named(NamedKey::End, NONE), Some(moves(Motion::RowEnd)));
}

#[test]
fn erases_by_character_word_and_row() {
    assert_eq!(named(NamedKey::Backspace, NONE), Some(erases(Motion::Left)));
    assert_eq!(named(NamedKey::Delete, NONE), Some(erases(Motion::Right)));
    assert_eq!(
        named(NamedKey::Backspace, ALT),
        Some(erases(Motion::WordLeft))
    );
    assert_eq!(
        named(NamedKey::Delete, ALT),
        Some(erases(Motion::WordRight))
    );
    assert_eq!(
        named(NamedKey::Backspace, COMMAND),
        Some(erases(Motion::RowStart))
    );
}

#[test]
fn command_v_pastes_whatever_the_case() {
    assert_eq!(character("v", COMMAND), Some(Intent::Paste));
    assert_eq!(character("V", COMMAND | SHIFT), Some(Intent::Paste));
}

#[test]
fn ignores_keys_that_mean_nothing_here() {
    assert_eq!(character("c", COMMAND), None);
    assert_eq!(character("a", CONTROL), None);
    assert_eq!(named(NamedKey::Enter, CONTROL), None);
    assert_eq!(intent(&Key::Named(NamedKey::Tab), Some("\t"), NONE), None);
    assert_eq!(
        intent(&Key::Named(NamedKey::Escape), Some("\u{1b}"), NONE),
        None
    );
    assert_eq!(named(NamedKey::F1, NONE), None);
}
