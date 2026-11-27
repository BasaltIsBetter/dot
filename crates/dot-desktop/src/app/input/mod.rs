use std::time::Instant;

use dot_api::{Connection, Image, ImageError, Secret, Text, Translator};

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Layout, Reticle};
use crate::conversation::{self, Edit, Intent, LONGEST_PROMPT, Stage};
use crate::event::{Keystroke, Wheel, WindowAction};
use crate::sessions::Hit;
use crate::shell::{pasted_image, pasted_text};
use crate::sign_in::{self, Input, KeyIntent, LONGEST_KEY};
use dot_transcript::to_f32;
use winit::keyboard::{Key, ModifiersState};

mod connection;
mod editing;
mod naming;
mod selection;

const WHEEL_LINE: f32 = 40.0;

pub(crate) use self::editing::Editing;

const ENOUGH_TO_SEE_TEXT: usize = 1;

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn follow_input(&mut self, action: WindowAction, now: Instant) {
        if self.edit_input(&action, now) {
            self.request_frame();
            return;
        }
        let layout = self.layout(now);
        match action {
            WindowAction::Rescale(scale_factor) => self.window.scale_factor = scale_factor,
            WindowAction::Focus => {
                self.window.focused = true;
                self.refresh_status();
            }
            WindowAction::Blur => self.window.focused = false,
            WindowAction::Leave => self.window.cursor = None,
            WindowAction::Modifiers(modifiers) => self.window.modifiers = modifiers,
            WindowAction::Key(keystroke) => self.type_key(&keystroke, now),
            WindowAction::Commit(text) if self.rename_text(&text) => {}
            WindowAction::Commit(text) if self.settings_screen.is_open() => {
                let typed = Keystroke {
                    key: Key::Character(text.clone().into()),
                    text: Some(text.into()),
                    modifiers: ModifiersState::empty(),
                };
                self.settings_key(&typed, now);
            }
            WindowAction::Commit(text) => match self.screen {
                Screen::Chat => self.sessions.active_mut().conversation.commit(text, now),
                Screen::SignIn => self.sign_in.input(Input::Type(Secret::new(text))),
            },
            WindowAction::Compose(text) => {
                if self.screen == Screen::Chat && !self.settings_screen.is_open() {
                    self.sessions.active_mut().conversation.compose(text);
                }
            }
            WindowAction::Point(position) => {
                self.window.cursor = Some(position);
                self.input_under_cursor(Input::Point);
                if self.window.pressing && self.screen == Screen::Chat {
                    let at = [to_f32(position.x), to_f32(position.y)];
                    let conversation = &mut self.sessions.active_mut().conversation;
                    conversation.select_drag(at, &layout, now);
                }
            }
            WindowAction::Press => {
                self.window.pressing = true;
                self.press(&layout, now);
            }
            WindowAction::Release => {
                self.window.pressing = false;
                self.sessions.active_mut().conversation.select_release(now);
            }
            WindowAction::Drop(path) => {
                if self.screen == Screen::Chat {
                    self.editing.read_image(path);
                }
            }
            WindowAction::Scroll(wheel) if self.settings_screen.is_open() => {
                self.settings_wheel(settings_pixels(wheel), now);
            }
            WindowAction::Scroll(_) if self.window.modifiers.super_key() => {
                self.open_settings(None, true, now);
            }
            WindowAction::Scroll(wheel) => {
                if let (Some(cursor), Screen::Chat) = (self.window.cursor, self.screen) {
                    let at = [to_f32(cursor.x), to_f32(cursor.y)];
                    let stage = Stage::of(&layout);
                    if !self.sessions.wheel_rail(wheel, at, (&stage, now)) {
                        let translator = self.translator;
                        let conversation = &mut self.sessions.active_mut().conversation;
                        conversation.scroll(wheel, (cursor, &layout), now, translator);
                    }
                }
            }
            _ => return,
        }
        self.request_frame();
    }

    fn type_key(&mut self, keystroke: &Keystroke, now: Instant) {
        if self.rename_key(keystroke, now) {
            return;
        }
        if self.screen == Screen::Chat && self.settings_screen.is_open() {
            self.settings_key(keystroke, now);
            return;
        }
        if self.screen == Screen::Chat && Self::toggles_settings(keystroke) {
            self.open_settings(None, false, now);
            return;
        }
        if self.screen == Screen::Chat && self.selection_key(keystroke, now) {
            return;
        }
        let asking = match self.screen {
            Screen::SignIn => return self.type_sign_in_key(keystroke),
            Screen::Chat => self.sessions.active().conversation.is_asking(),
        };
        if asking {
            self.answer(keystroke, now);
            return;
        }

        let Keystroke {
            key,
            text,
            modifiers,
        } = keystroke;
        let intent = conversation::intent(key, text.as_deref(), *modifiers);
        if intent == Some(Intent::Paste)
            && let Some(bytes) = pasted_image()
            && (bytes.is_ok() || pasted_text(ENOUGH_TO_SEE_TEXT).is_none())
        {
            let name = self.translator.translate(Text::ImagePasted).into_owned();
            self.attach(&name, bytes, now);
            return;
        }
        if let Some(Intent::Switch(step)) = intent {
            self.sessions.step(step);
            return;
        }
        let translator = self.translator;
        let conversation = &mut self.sessions.active_mut().conversation;
        if let Some(Intent::Edit(Edit::Move(motion))) = intent
            && conversation.recall(motion, &self.recall, now)
        {
            return;
        }
        let command = intent.and_then(|intent| {
            conversation.type_key(intent, now, || pasted_text(LONGEST_PROMPT), translator)
        });
        if let Some(command) = command {
            self.run(command, now);
        }
    }

    fn type_sign_in_key(&mut self, keystroke: &Keystroke) {
        let Keystroke {
            key,
            text,
            modifiers,
        } = keystroke;
        match sign_in::intent(key, text.as_deref(), *modifiers) {
            Some(KeyIntent::Input(input)) => {
                self.sign_in.input(input);
                self.enter_chat_once_connected();
            }
            Some(KeyIntent::Paste) => {
                if let Some(text) = pasted_text(LONGEST_KEY) {
                    self.sign_in.input(Input::Paste(Secret::new(text)));
                }
            }
            None => {}
        }
    }

    fn attach(&mut self, name: &str, bytes: Result<Vec<u8>, ImageError>, now: Instant) {
        if self.screen != Screen::Chat {
            return;
        }
        let translator = self.translator;
        let conversation = &mut self.sessions.active_mut().conversation;
        let attached = bytes
            .and_then(Image::from_bytes)
            .and_then(|image| conversation.attach(name, image, translator));
        if let Err(error) = attached {
            let notice = translator.translate(error.text());
            conversation.notice(&notice, now);
        }
    }

    fn press(&mut self, layout: &Layout, now: Instant) {
        if self.screen != Screen::Chat {
            self.input_under_cursor(Input::Press);
            return;
        }
        let Some(cursor) = self.window.cursor else {
            return;
        };
        let at = [to_f32(cursor.x), to_f32(cursor.y)];
        let target = self
            .strip
            .as_ref()
            .and_then(|strip| strip.target_at(layout, at));
        if let Some(target) = target {
            self.strip_target(target, now);
            return;
        }
        if self.settings_screen.is_open() {
            self.settings_press(at, layout, now);
            return;
        }
        let hit = self.rail(layout, now).and_then(|rail| rail.hit(at));
        if self.card_press(hit, now) {
            return;
        }
        match hit {
            Some(Hit::Switch(index)) => self.sessions.switch_to(index),
            Some(Hit::Close(index)) => self.sessions.close(index),
            Some(Hit::Open) => self.open_session(),
            Some(Hit::Name(_)) | None => {
                let clicks = self.count_click(at, now);
                let conversation = &mut self.sessions.active_mut().conversation;
                if conversation.press_marker(at, layout, now, self.translator) {
                    return;
                }
                match conversation.file_under(cursor, layout, now) {
                    Some(path) => self.open_file(path),
                    None if conversation.select_press(at, clicks, layout, now) => {}
                    None => conversation.press(cursor, layout, now),
                }
            }
        }
    }

    fn input_under_cursor(&mut self, input: fn(Connection) -> Input) {
        if !matches!(self.screen, Screen::SignIn) {
            return;
        }
        let layout = Layout::zoomed_to(self.window.size, self.window.scale_factor, self.size());
        let method = self
            .window
            .cursor
            .and_then(|cursor| layout.line_at(cursor))
            .and_then(|row| self.sign_in.method_at(row));
        if let Some(method) = method {
            self.sign_in.input(input(method));
        }
    }

    pub(super) fn reticle(&self, layout: &Layout, now: Instant) -> Option<Reticle> {
        if !self.window.focused {
            return None;
        }
        let cursor = self.window.cursor?;
        let over_text = self.screen == Screen::Chat
            && !self.editing.is_open()
            && !self.settings_screen.is_open()
            && self.sessions.active().conversation.is_over_text(
                [to_f32(cursor.x), to_f32(cursor.y)],
                layout,
                now,
            );
        if over_text {
            return None;
        }
        Some(Reticle::at(cursor, self.window.scale_factor))
    }
}

fn settings_pixels(wheel: Wheel) -> f32 {
    match wheel {
        Wheel::Lines(lines) => -lines * WHEEL_LINE,
        Wheel::Pixels(pixels) => -to_f32(pixels),
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/input/mod.rs"]
mod tests;
