use std::time::Instant;

use winit::keyboard::{Key, NamedKey};

use crate::app::{DesktopApp, Parts};
use crate::event::Keystroke;
use crate::sessions::{Hit, NameField, shown_name};

impl<P: Parts> DesktopApp<'_, P> {
    pub(in crate::app) fn card_press(&mut self, hit: Option<Hit>, now: Instant) -> bool {
        let renamed = match (hit, &self.renaming) {
            (Some(Hit::Name(index)), Some(field)) => self.sessions.key_of(index) != Some(field.key),
            (_, renaming) => renaming.is_some(),
        };
        if renamed {
            self.finish_renaming(true, now);
        }
        if !matches!(hit, Some(Hit::Hue(_) | Hit::Pick(..))) {
            self.picking = None;
        }
        match hit {
            Some(Hit::Hue(index)) => {
                let key = self.sessions.key_of(index);
                self.picking = if self.picking == key { None } else { key };
                true
            }
            Some(Hit::Pick(index, hue)) => {
                self.sessions.paint(index, hue);
                self.picking = None;
                true
            }
            Some(Hit::Name(index)) => {
                self.start_renaming(index);
                true
            }
            _ => false,
        }
    }

    pub(in crate::app) fn rename_key(&mut self, keystroke: &Keystroke, now: Instant) -> bool {
        let Some(field) = self.renaming.as_mut() else {
            return false;
        };
        match &keystroke.key {
            Key::Named(NamedKey::Enter) => self.finish_renaming(true, now),
            Key::Named(NamedKey::Escape) => self.finish_renaming(false, now),
            Key::Named(NamedKey::Backspace) => field.erase(),
            Key::Named(NamedKey::ArrowLeft) => field.left(),
            Key::Named(NamedKey::ArrowRight) => field.right(),
            _ if keystroke.modifiers.super_key() || keystroke.modifiers.control_key() => {}
            _ => {
                if let Some(text) = keystroke.text.as_deref() {
                    field.type_text(text);
                }
            }
        }
        true
    }

    pub(in crate::app) fn rename_text(&mut self, text: &str) -> bool {
        let Some(field) = self.renaming.as_mut() else {
            return false;
        };
        field.type_text(text);
        true
    }

    fn start_renaming(&mut self, index: usize) {
        let Some(key) = self.sessions.key_of(index) else {
            return;
        };
        if self.renaming.as_ref().is_some_and(|field| field.key == key) {
            return;
        }
        let Some(opened) = self.sessions.by_key(key) else {
            return;
        };
        let shown = shown_name(opened, self.status_bar.project(), self.translator);
        self.renaming = Some(NameField::of(key, &shown));
    }

    fn finish_renaming(&mut self, keep: bool, now: Instant) {
        let Some(field) = self.renaming.take() else {
            return;
        };
        if !keep {
            return;
        }
        let Some(opened) = self.sessions.by_key(field.key) else {
            return;
        };
        let shown = shown_name(opened, self.status_bar.project(), self.translator);
        self.sessions.rename(&field, &shown, now);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/input/naming.rs"]
mod tests;
