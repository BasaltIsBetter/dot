use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::conversation::prompter::draft::Edit;
use crate::conversation::prompter::motion::Motion;

const PASTE: &str = "v";
const START_OVER: &str = "n";
const NEW_LINE: &str = "\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Intent {
    Edit(Edit),
    Send,
    Stop,
    Paste,
    StartOver,
}

pub(crate) fn intent(key: &Key, text: Option<&str>, modifiers: ModifiersState) -> Option<Intent> {
    if modifiers.control_key() {
        return None;
    }
    if modifiers.super_key() {
        return command_intent(key);
    }

    let by_word = modifiers.alt_key();
    let edit = match key {
        Key::Named(NamedKey::Enter) if by_word || modifiers.shift_key() => {
            Edit::Insert(NEW_LINE.to_owned())
        }
        Key::Named(NamedKey::Enter) => return Some(Intent::Send),
        Key::Named(NamedKey::Backspace) => Edit::Erase(leftwards(by_word)),
        Key::Named(NamedKey::Delete) => Edit::Erase(rightwards(by_word)),
        Key::Named(NamedKey::ArrowLeft) => Edit::Move(leftwards(by_word)),
        Key::Named(NamedKey::ArrowRight) => Edit::Move(rightwards(by_word)),
        Key::Named(NamedKey::ArrowUp) => Edit::Move(Motion::Up),
        Key::Named(NamedKey::ArrowDown) => Edit::Move(Motion::Down),
        Key::Named(NamedKey::Home) => Edit::Move(Motion::RowStart),
        Key::Named(NamedKey::End) => Edit::Move(Motion::RowEnd),
        Key::Named(NamedKey::Escape) => return Some(Intent::Stop),
        Key::Named(NamedKey::Tab) => return None,
        _ => Edit::Insert(text?.to_owned()),
    };
    Some(Intent::Edit(edit))
}

fn command_intent(key: &Key) -> Option<Intent> {
    let edit = match key {
        Key::Character(character) if character.eq_ignore_ascii_case(PASTE) => {
            return Some(Intent::Paste);
        }
        Key::Character(character) if character.eq_ignore_ascii_case(START_OVER) => {
            return Some(Intent::StartOver);
        }
        Key::Named(NamedKey::ArrowLeft) => Edit::Move(Motion::RowStart),
        Key::Named(NamedKey::ArrowRight) => Edit::Move(Motion::RowEnd),
        Key::Named(NamedKey::ArrowUp) => Edit::Move(Motion::Start),
        Key::Named(NamedKey::ArrowDown) => Edit::Move(Motion::End),
        Key::Named(NamedKey::Backspace) => Edit::Erase(Motion::RowStart),
        _ => return None,
    };
    Some(Intent::Edit(edit))
}

const fn leftwards(by_word: bool) -> Motion {
    if by_word {
        Motion::WordLeft
    } else {
        Motion::Left
    }
}

const fn rightwards(by_word: bool) -> Motion {
    if by_word {
        Motion::WordRight
    } else {
        Motion::Right
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/keys.rs"]
mod tests;
