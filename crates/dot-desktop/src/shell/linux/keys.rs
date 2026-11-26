use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::event::Keystroke;

pub(crate) const COPY_KEY: &str = "ctrl+C";
const QUIT: &str = "q";

/// The modifiers as the rest of Dot reads them: ctrl is the command key of a Linux desktop, so it
/// takes the place `⌘` has on the Mac.
pub(crate) fn native_modifiers(modifiers: ModifiersState) -> ModifiersState {
    if !modifiers.control_key() {
        return modifiers;
    }
    (modifiers - ModifiersState::CONTROL) | ModifiersState::SUPER
}

/// The keystroke as the rest of Dot reads it: ctrl moves and erases by word where the Mac uses
/// `⌥`, and a key that writes nothing on screen carries no text.
pub(crate) fn native_keystroke(keystroke: Keystroke) -> Keystroke {
    let Keystroke {
        key,
        text,
        modifiers,
    } = keystroke;
    let by_word = modifiers.super_key()
        && matches!(
            key,
            Key::Named(
                NamedKey::ArrowLeft | NamedKey::ArrowRight | NamedKey::Backspace | NamedKey::Delete
            )
        );
    let writes = !modifiers.alt_key() && !modifiers.super_key();
    Keystroke {
        key,
        text: text.filter(|text| writes && !text.chars().any(char::is_control)),
        modifiers: if by_word {
            (modifiers - ModifiersState::SUPER) | ModifiersState::ALT
        } else {
            modifiers
        },
    }
}

/// Whether the keystroke quits Dot: without a menu, ctrl+Q is Dot's own to follow.
pub(crate) fn quits(keystroke: &Keystroke) -> bool {
    keystroke.modifiers.super_key()
        && matches!(&keystroke.key, Key::Character(character) if character.eq_ignore_ascii_case(QUIT))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/shell/linux/keys.rs"]
mod tests;
