use std::time::Instant;

use winit::keyboard::SmolStr;

use super::*;
use crate::editor::tests::{after, editor};

fn with(modifiers: ModifiersState, key: &Key) -> Option<Gesture> {
    gesture(key, None, modifiers)
}

fn named(key: NamedKey) -> Key {
    Key::Named(key)
}

fn character(text: &str) -> Key {
    Key::Character(SmolStr::new(text))
}

#[test]
fn reads_each_key_as_a_gesture_of_the_editor() {
    let none = ModifiersState::empty();
    let shift = ModifiersState::SHIFT;
    let alt = ModifiersState::ALT;

    assert_eq!(
        with(none, &named(NamedKey::Enter)),
        Some(Gesture::Insert("\n".to_owned()))
    );
    assert_eq!(
        with(none, &named(NamedKey::Tab)),
        Some(Gesture::Insert("    ".to_owned()))
    );
    assert_eq!(with(shift, &named(NamedKey::Tab)), None);
    assert_eq!(gesture(&named(NamedKey::Tab), Some("\t"), shift), None);
    assert_eq!(
        with(none, &named(NamedKey::Backspace)),
        Some(Gesture::Erase(Motion::Left))
    );
    assert_eq!(
        with(alt, &named(NamedKey::Backspace)),
        Some(Gesture::Erase(Motion::WordLeft))
    );
    assert_eq!(
        with(none, &named(NamedKey::Delete)),
        Some(Gesture::Erase(Motion::Right))
    );
    assert_eq!(
        with(alt, &named(NamedKey::Delete)),
        Some(Gesture::Erase(Motion::WordRight))
    );
    assert_eq!(
        with(shift, &named(NamedKey::ArrowLeft)),
        Some(Gesture::Go(Motion::Left, true))
    );
    assert_eq!(
        with(alt, &named(NamedKey::ArrowRight)),
        Some(Gesture::Go(Motion::WordRight, false))
    );
    assert_eq!(
        with(none, &named(NamedKey::ArrowUp)),
        Some(Gesture::Go(Motion::Up, false))
    );
    assert_eq!(
        with(shift, &named(NamedKey::ArrowDown)),
        Some(Gesture::Go(Motion::Down, true))
    );
    assert_eq!(
        with(none, &named(NamedKey::Home)),
        Some(Gesture::Go(Motion::RowStart, false))
    );
    assert_eq!(
        with(shift, &named(NamedKey::End)),
        Some(Gesture::Go(Motion::RowEnd, true))
    );
    assert_eq!(with(none, &named(NamedKey::Escape)), Some(Gesture::Close));
    assert_eq!(
        gesture(&character("é"), Some("é"), none),
        Some(Gesture::Insert("é".to_owned()))
    );
    assert_eq!(gesture(&character("é"), None, none), None);
    assert_eq!(with(ModifiersState::CONTROL, &character("a")), None);
}

#[test]
fn reads_the_command_keys_of_macos() {
    let command = ModifiersState::SUPER;
    let command_shift = ModifiersState::SUPER | ModifiersState::SHIFT;

    assert_eq!(with(command, &character("z")), Some(Gesture::Undo));
    assert_eq!(with(command_shift, &character("Z")), Some(Gesture::Redo));
    assert_eq!(with(command, &character("a")), Some(Gesture::SelectAll));
    assert_eq!(with(command, &character("c")), Some(Gesture::Copy));
    assert_eq!(with(command, &character("x")), Some(Gesture::Cut));
    assert_eq!(with(command, &character("v")), Some(Gesture::Paste));
    assert_eq!(
        with(command, &named(NamedKey::ArrowLeft)),
        Some(Gesture::Go(Motion::RowStart, false))
    );
    assert_eq!(
        with(command_shift, &named(NamedKey::ArrowRight)),
        Some(Gesture::Go(Motion::RowEnd, true))
    );
    assert_eq!(
        with(command, &named(NamedKey::Backspace)),
        Some(Gesture::Erase(Motion::RowStart))
    );
    assert_eq!(with(command, &character("q")), None);
}

#[test]
fn says_what_each_gesture_changed_copied_or_closed() {
    let start = Instant::now();
    let mut editor = editor(start);
    let nothing = || None;

    assert_eq!(
        editor.act(Gesture::Go(Motion::RowEnd, true), start, nothing),
        Acted::default()
    );
    assert_eq!(
        editor.act(Gesture::Copy, start, nothing),
        Acted {
            copied: Some("a".to_owned()),
            ..Acted::default()
        }
    );
    let cut = editor.act(Gesture::Cut, start, nothing);
    assert_eq!(
        cut,
        Acted {
            changed: true,
            copied: Some("a".to_owned()),
            closed: false
        }
    );
    assert_eq!(editor.text(), "\nB\nc");
    assert!(
        editor
            .act(Gesture::Paste, start, || Some("z".to_owned()))
            .changed
    );
    assert!(!editor.act(Gesture::Paste, start, nothing).changed);
    assert!(
        editor
            .act(
                Gesture::Insert("y".to_owned()),
                after(start, 2_000),
                nothing
            )
            .changed
    );
    assert!(
        editor
            .act(Gesture::Erase(Motion::Left), after(start, 4_000), nothing)
            .changed
    );
    assert!(
        editor
            .act(Gesture::Undo, after(start, 4_000), nothing)
            .changed
    );
    assert!(
        editor
            .act(Gesture::Redo, after(start, 4_000), nothing)
            .changed
    );
    assert!(!editor.act(Gesture::SelectAll, start, nothing).changed);
    assert_eq!(
        editor.act(Gesture::Close, start, nothing),
        Acted {
            closed: true,
            ..Acted::default()
        }
    );
}

#[test]
fn counts_the_lines_you_changed_as_you_type() {
    let start = Instant::now();
    let mut editor = editor(start);

    editor.act(Gesture::Insert("x".to_owned()), start, || None);

    assert_eq!(editor.mine.count(), 1);
    assert!(editor.mine.changed[0]);
}

#[test]
fn walks_back_and_forth_between_your_changes_claude_s_and_before_claude() {
    let start = Instant::now();
    let mut editor = editor(start);
    editor.act(Gesture::Insert("x".to_owned()), start, || None);

    assert!(editor.step(true, start));
    assert_eq!(editor.text(), "a\nB\nc");
    assert!(editor.step(true, start));
    assert_eq!(editor.text(), "a\nb\nc");
    assert!(!editor.step(true, start));
    assert!(editor.step(false, start));
    assert_eq!(editor.text(), "a\nB\nc");
    assert!(editor.step(false, start));
    assert_eq!(editor.text(), "xa\nB\nc");
    assert!(!editor.step(false, start));
    assert_eq!(editor.mine.count(), 1);
}
