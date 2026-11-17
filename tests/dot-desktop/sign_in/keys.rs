use winit::keyboard::SmolStr;

use super::*;

fn named(key: NamedKey) -> Option<KeyIntent> {
    intent(&Key::Named(key), None, ModifiersState::empty())
}

fn character(text: &str, modifiers: ModifiersState) -> Option<KeyIntent> {
    intent(&Key::Character(SmolStr::new(text)), Some(text), modifiers)
}

#[test]
fn arrows_move_the_selection() {
    assert_eq!(
        named(NamedKey::ArrowUp),
        Some(KeyIntent::Input(Input::Previous))
    );
    assert_eq!(
        named(NamedKey::ArrowDown),
        Some(KeyIntent::Input(Input::Next))
    );
    assert_eq!(named(NamedKey::Tab), Some(KeyIntent::Input(Input::Next)));
}

#[test]
fn return_escape_and_backspace_confirm_go_back_and_erase() {
    assert_eq!(
        named(NamedKey::Enter),
        Some(KeyIntent::Input(Input::Confirm))
    );
    assert_eq!(named(NamedKey::Escape), Some(KeyIntent::Input(Input::Back)));
    assert_eq!(
        named(NamedKey::Backspace),
        Some(KeyIntent::Input(Input::Erase))
    );
}

#[test]
fn a_character_types_its_text() {
    assert_eq!(
        character("k", ModifiersState::SHIFT),
        Some(KeyIntent::Input(Input::Type("k".to_owned())))
    );
}

#[test]
fn command_v_pastes_whatever_the_case() {
    assert_eq!(
        character("v", ModifiersState::SUPER),
        Some(KeyIntent::Paste)
    );
    assert_eq!(
        character("V", ModifiersState::SUPER | ModifiersState::SHIFT),
        Some(KeyIntent::Paste)
    );
}

#[test]
fn other_shortcuts_type_nothing() {
    assert_eq!(character("c", ModifiersState::SUPER), None);
    assert_eq!(character("v", ModifiersState::CONTROL), None);
    assert_eq!(named(NamedKey::F1), None);
}
