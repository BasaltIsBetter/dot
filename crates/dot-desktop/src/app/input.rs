use std::time::Instant;

use dot_api::{Connector, Session, SessionStatus, Translator};

use crate::app::DesktopApp;
use crate::canvas::{Canvas, Layout, Reticle};
use crate::conversation;
use crate::event::{Keystroke, WindowAction};
use crate::shell::pasted_text;
use crate::sign_in::{self, Input, KeyIntent, Method, Report};
use crate::worker::Mailbox;

impl<T, K, S, M, C> DesktopApp<'_, T, K, S, M, C>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    M: Mailbox<Report> + Mailbox<SessionStatus>,
    C: Canvas,
{
    pub(super) fn follow_input(&mut self, action: WindowAction, now: Instant) {
        let layout = self.layout();
        match action {
            WindowAction::Rescale(scale_factor) => self.window.scale_factor = scale_factor,
            WindowAction::Focus => {
                self.window.focused = true;
                self.status_bar.refresh();
            }
            WindowAction::Blur => self.window.focused = false,
            WindowAction::Leave => self.window.cursor = None,
            WindowAction::Modifiers(modifiers) => self.window.modifiers = modifiers,
            WindowAction::Key(keystroke) => self.type_key(&keystroke, &layout, now),
            WindowAction::Commit(text) if self.is_connected() => {
                self.conversation.commit(text, &layout);
            }
            WindowAction::Commit(text) => self.sign_in.input(Input::Type(text)),
            WindowAction::Compose(text) if self.is_connected() => self.conversation.compose(text),
            WindowAction::Point(position) => {
                self.window.cursor = Some(position);
                self.input_under_cursor(Input::Point);
            }
            WindowAction::Press => self.press(&layout),
            WindowAction::Scroll(wheel) => {
                if let Some(cursor) = self.window.cursor.filter(|_| self.is_connected()) {
                    self.conversation.scroll(wheel, cursor, &layout);
                }
            }
            _ => return,
        }
        self.refresh(now);
    }

    fn type_key(&mut self, keystroke: &Keystroke, layout: &Layout, now: Instant) {
        let Keystroke {
            key,
            text,
            modifiers,
        } = keystroke;
        if self.is_connected() {
            if let Some(intent) = conversation::intent(key, text.as_deref(), *modifiers) {
                self.conversation.type_key(intent, layout, now, pasted_text);
            }
            return;
        }

        match sign_in::intent(key, text.as_deref(), *modifiers) {
            Some(KeyIntent::Input(input)) => self.sign_in.input(input),
            Some(KeyIntent::Paste) => {
                if let Some(text) = pasted_text() {
                    self.sign_in.input(Input::Paste(text));
                }
            }
            None => {}
        }
    }

    fn press(&mut self, layout: &Layout) {
        if !self.is_connected() {
            self.input_under_cursor(Input::Press);
            return;
        }
        if let Some(cursor) = self.window.cursor {
            self.conversation.press(cursor, layout);
        }
    }

    fn input_under_cursor(&mut self, input: fn(Method) -> Input) {
        let layout = self.layout();
        let method = self
            .window
            .cursor
            .and_then(|cursor| layout.line_at(cursor))
            .and_then(|row| self.sign_in.method_at(row));
        if let Some(method) = method {
            self.sign_in.input(input(method));
        }
    }

    pub(super) fn reticle(&self) -> Option<Reticle> {
        if !self.window.focused {
            return None;
        }
        let cursor = self.window.cursor?;
        Some(Reticle::at(cursor, self.window.scale_factor))
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/input.rs"]
mod tests;
