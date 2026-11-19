use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use dot_api::{Connection, Image, ImageError, Secret, Text, Translator};

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Layout, Reticle};
use crate::conversation::{self, Command, Intent};
use crate::event::{Keystroke, WindowAction};
use crate::shell::{pasted_image, pasted_text};
use crate::sign_in::{self, Input, KeyIntent};

impl<P: Parts> DesktopApp<'_, P> {
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
            WindowAction::Commit(text) => match &mut self.screen {
                Screen::Chat(conversation) => conversation.commit(text, &layout),
                Screen::SignIn => self.sign_in.input(Input::Type(Secret::new(text))),
            },
            WindowAction::Compose(text) => {
                if let Screen::Chat(conversation) = &mut self.screen {
                    conversation.compose(text);
                }
            }
            WindowAction::Point(position) => {
                self.window.cursor = Some(position);
                self.input_under_cursor(Input::Point);
            }
            WindowAction::Press => self.press(&layout),
            WindowAction::Drop(path) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                self.attach(&name, read_image(&path), now);
            }
            WindowAction::Scroll(wheel) => {
                if let (Some(cursor), Screen::Chat(conversation)) =
                    (self.window.cursor, &mut self.screen)
                {
                    conversation.scroll(wheel, cursor, &layout, now);
                }
            }
            _ => return,
        }
        self.request_frame();
    }

    fn type_key(&mut self, keystroke: &Keystroke, layout: &Layout, now: Instant) {
        let asking = match &self.screen {
            Screen::SignIn => return self.type_sign_in_key(keystroke),
            Screen::Chat(conversation) => conversation.is_asking(),
        };
        if asking {
            self.answer(keystroke, layout, now);
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
        let Screen::Chat(conversation) = &mut self.screen else {
            return;
        };
        let command =
            intent.and_then(|intent| conversation.type_key(intent, layout, now, pasted_text));
        match command {
            Some(Command::Ask(prompt)) => self.replies.ask(prompt),
            Some(Command::Stop) => self.stop_reply(now),
            Some(Command::StartOver) => self.start_over(now),
            None => {}
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
        let Screen::Chat(conversation) = &mut self.screen else {
            return;
        };
        let attached = bytes
            .and_then(Image::from_bytes)
            .and_then(|image| conversation.attach(name, image, self.translator));
        if let Err(error) = attached {
            let notice = self.translator.translate(error.text());
            conversation.notice(&notice, now);
        }
    }

    fn press(&mut self, layout: &Layout) {
        let Screen::Chat(conversation) = &mut self.screen else {
            self.input_under_cursor(Input::Press);
            return;
        };
        if let Some(cursor) = self.window.cursor {
            conversation.press(cursor, layout);
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
#[path = "../../../../tests/dot-desktop/app/input.rs"]
mod tests;
