use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::sign_in::screen::Input;

const PASTE: &str = "v";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyIntent {
    Input(Input),
    Paste,
}

pub(crate) fn intent(
    key: &Key,
    text: Option<&str>,
    modifiers: ModifiersState,
) -> Option<KeyIntent> {
    if modifiers.super_key() || modifiers.control_key() {
        let pastes = modifiers.super_key()
            && matches!(key, Key::Character(character) if character.eq_ignore_ascii_case(PASTE));
        return pastes.then_some(KeyIntent::Paste);
    }

    let input = match key {
        Key::Named(NamedKey::ArrowUp) => Input::Previous,
        Key::Named(NamedKey::ArrowDown | NamedKey::Tab) => Input::Next,
        Key::Named(NamedKey::Enter) => Input::Confirm,
        Key::Named(NamedKey::Escape) => Input::Back,
        Key::Named(NamedKey::Backspace) => Input::Erase,
        _ => Input::Type(text?.to_owned()),
    };
    Some(KeyIntent::Input(input))
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sign_in/keys.rs"]
mod tests;
