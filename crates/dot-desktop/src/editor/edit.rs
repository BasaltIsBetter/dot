use std::time::Instant;

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::conversation::Motion;
use crate::editor::Editor;
use dot_transcript::NEW_LINE;

const UNDO: &str = "z";
const SELECT_ALL: &str = "a";
const COPY: &str = "c";
const CUT: &str = "x";
const PASTE: &str = "v";
const INDENT: &str = "    ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Gesture {
    Insert(String),
    Erase(Motion),
    Go(Motion, bool),
    SelectAll,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Close,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Acted {
    pub(crate) changed: bool,
    pub(crate) copied: Option<String>,
    pub(crate) closed: bool,
}

pub(crate) fn gesture(key: &Key, text: Option<&str>, modifiers: ModifiersState) -> Option<Gesture> {
    if modifiers.control_key() {
        return None;
    }
    let extend = modifiers.shift_key();
    if modifiers.super_key() {
        return command(key, extend);
    }
    let by_word = modifiers.alt_key();
    let gesture = match key {
        Key::Named(NamedKey::Enter) => Gesture::Insert(NEW_LINE.to_string()),
        Key::Named(NamedKey::Tab) if !extend => Gesture::Insert(INDENT.to_owned()),
        Key::Named(NamedKey::Tab) => return None,
        Key::Named(NamedKey::Backspace) => Gesture::Erase(left(by_word)),
        Key::Named(NamedKey::Delete) => Gesture::Erase(right(by_word)),
        Key::Named(NamedKey::ArrowLeft) => Gesture::Go(left(by_word), extend),
        Key::Named(NamedKey::ArrowRight) => Gesture::Go(right(by_word), extend),
        Key::Named(NamedKey::ArrowUp) => Gesture::Go(Motion::Up, extend),
        Key::Named(NamedKey::ArrowDown) => Gesture::Go(Motion::Down, extend),
        Key::Named(NamedKey::Home) => Gesture::Go(Motion::RowStart, extend),
        Key::Named(NamedKey::End) => Gesture::Go(Motion::RowEnd, extend),
        Key::Named(NamedKey::Escape) => Gesture::Close,
        _ => Gesture::Insert(text?.to_owned()),
    };
    Some(gesture)
}

fn command(key: &Key, extend: bool) -> Option<Gesture> {
    let gesture = match key {
        Key::Character(character) if character.eq_ignore_ascii_case(UNDO) && extend => {
            Gesture::Redo
        }
        Key::Character(character) if character.eq_ignore_ascii_case(UNDO) => Gesture::Undo,
        Key::Character(character) if character.eq_ignore_ascii_case(SELECT_ALL) => {
            Gesture::SelectAll
        }
        Key::Character(character) if character.eq_ignore_ascii_case(COPY) => Gesture::Copy,
        Key::Character(character) if character.eq_ignore_ascii_case(CUT) => Gesture::Cut,
        Key::Character(character) if character.eq_ignore_ascii_case(PASTE) => Gesture::Paste,
        Key::Named(NamedKey::ArrowLeft) => Gesture::Go(Motion::RowStart, extend),
        Key::Named(NamedKey::ArrowRight) => Gesture::Go(Motion::RowEnd, extend),
        Key::Named(NamedKey::Backspace) => Gesture::Erase(Motion::RowStart),
        _ => return None,
    };
    Some(gesture)
}

const fn left(by_word: bool) -> Motion {
    if by_word {
        Motion::WordLeft
    } else {
        Motion::Left
    }
}

const fn right(by_word: bool) -> Motion {
    if by_word {
        Motion::WordRight
    } else {
        Motion::Right
    }
}

impl Editor {
    pub(crate) fn act(
        &mut self,
        gesture: Gesture,
        now: Instant,
        clipboard: impl FnOnce() -> Option<String>,
    ) -> Acted {
        let mut acted = Acted::default();
        match gesture {
            Gesture::Insert(text) => acted.changed = self.buffer.insert(&text, now),
            Gesture::Erase(motion) => acted.changed = self.buffer.erase(motion, now),
            Gesture::Go(motion, extend) => self.buffer.go(motion, extend, now),
            Gesture::SelectAll => self.buffer.select_all(),
            Gesture::Undo => acted.changed = self.buffer.undo(now),
            Gesture::Redo => acted.changed = self.buffer.redo(now),
            Gesture::Copy => acted.copied = self.buffer.selected().map(str::to_owned),
            Gesture::Cut => {
                acted.copied = self.buffer.cut(now);
                acted.changed = acted.copied.is_some();
            }
            Gesture::Paste => {
                acted.changed = clipboard().is_some_and(|text| self.buffer.insert(&text, now));
            }
            Gesture::Close => acted.closed = true,
        }
        self.settle(acted.changed);
        acted
    }

    pub(crate) fn step(&mut self, back: bool, now: Instant) -> bool {
        let current = self.buffer.text();
        let toward = if back {
            self.versions.back(current)
        } else {
            self.versions.forward(current)
        };
        let Some(toward) = toward else {
            return false;
        };
        if back {
            self.versions.leave(self.buffer.text());
        }
        let text = self.versions.text(toward).to_owned();
        let changed = self.buffer.replace_all(&text, now);
        self.settle(changed);
        changed
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/editor/edit.rs"]
mod tests;
