use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use dot_api::{Connection, Image, ImageError, Secret, Text, Translator};

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Layout, Reticle};
use crate::conversation::{self, Intent};
use crate::event::{Keystroke, WindowAction};
use crate::sessions::Hit;
use crate::shell::{pasted_image, pasted_text};
use crate::sign_in::{self, Input, KeyIntent};
use dot_transcript::to_f32;

mod editing;

pub(crate) use self::editing::Editing;

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn follow_input(&mut self, action: WindowAction, now: Instant) {
        if self.edit_input(&action, now) {
            self.request_frame();
            return;
        }
        let layout = self.layout();
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
            WindowAction::Commit(text) => match self.screen {
                Screen::Chat => self.sessions.active_mut().conversation.commit(text, now),
                Screen::SignIn => self.sign_in.input(Input::Type(Secret::new(text))),
            },
            WindowAction::Compose(text) => {
                if self.screen == Screen::Chat {
                    self.sessions.active_mut().conversation.compose(text);
                }
            }
            WindowAction::Point(position) => {
                self.window.cursor = Some(position);
                self.input_under_cursor(Input::Point);
            }
            WindowAction::Press => self.press(&layout, now),
            WindowAction::Drop(path) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                self.attach(&name, read_image(&path), now);
            }
            WindowAction::Scroll(wheel) => {
                if let (Some(cursor), Screen::Chat) = (self.window.cursor, self.screen) {
                    let translator = self.translator;
                    let conversation = &mut self.sessions.active_mut().conversation;
                    conversation.scroll(wheel, (cursor, &layout), now, translator);
                }
            }
            _ => return,
        }
        self.request_frame();
    }

    fn type_key(&mut self, keystroke: &Keystroke, now: Instant) {
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
        {
            let name = self.translator.translate(Text::ImagePasted).into_owned();
            self.attach(&name, bytes, now);
            return;
        }
        if let Some(Intent::Switch(step)) = intent {
            self.sessions.step(step, now);
            return;
        }
        let translator = self.translator;
        let conversation = &mut self.sessions.active_mut().conversation;
        let command =
            intent.and_then(|intent| conversation.type_key(intent, now, pasted_text, translator));
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
            Some(KeyIntent::Input(input)) => self.sign_in.input(input),
            Some(KeyIntent::Paste) => {
                if let Some(text) = pasted_text() {
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
        match self.rail(layout, now).and_then(|rail| rail.hit(at)) {
            Some(Hit::Switch(index)) => self.sessions.switch_to(index, now),
            Some(Hit::Close(index)) => self.sessions.close(index, now),
            Some(Hit::Open) => self.open_session(now),
            None => {
                let conversation = &mut self.sessions.active_mut().conversation;
                match conversation.file_under(cursor, layout, now) {
                    Some(path) => self.open_file(path),
                    None => conversation.press(cursor, layout, now),
                }
            }
        }
    }

    fn input_under_cursor(&mut self, input: fn(Connection) -> Input) {
        if !matches!(self.screen, Screen::SignIn) {
            return;
        }
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

fn read_image(path: &Path) -> Result<Vec<u8>, ImageError> {
    let mut bytes = Vec::new();
    let longest = u64::try_from(Image::LARGEST).unwrap_or(u64::MAX);
    let read = File::open(path).and_then(|file| file.take(longest + 1).read_to_end(&mut bytes));
    match read {
        Ok(_) => Ok(bytes),
        Err(_) => Err(ImageError::Unsupported),
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/input/mod.rs"]
mod tests;
