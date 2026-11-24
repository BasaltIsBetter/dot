use std::time::{Duration, Instant};

use winit::keyboard::{Key, NamedKey};

use crate::app::{DesktopApp, Parts};
use crate::event::Keystroke;
use crate::shell::copy_text;

const COPY_KEY: &str = "c";
const DOUBLE_CLICK: Duration = Duration::from_millis(500);
const CLICK_REACH: f32 = 4.0;
const MOST_CLICKS: u8 = 3;

impl<P: Parts> DesktopApp<'_, P> {
    pub(in crate::app) fn selection_key(&mut self, keystroke: &Keystroke, now: Instant) -> bool {
        let conversation = &mut self.sessions.active_mut().conversation;
        match &keystroke.key {
            Key::Named(NamedKey::Escape) if !keystroke.modifiers.super_key() => {
                conversation.clear_selection()
            }
            Key::Character(character)
                if keystroke.modifiers.super_key() && character == COPY_KEY =>
            {
                let Some(text) = conversation.selected_text() else {
                    return false;
                };
                copy_text(&text);
                conversation.mark_copied(now);
                true
            }
            _ => false,
        }
    }

    pub(in crate::app) fn count_click(&mut self, at: [f32; 2], now: Instant) -> u8 {
        let clicks = match self.window.last_press {
            Some((before, [x, y], clicks))
                if now.saturating_duration_since(before) < DOUBLE_CLICK
                    && (at[0] - x).abs() <= CLICK_REACH
                    && (at[1] - y).abs() <= CLICK_REACH =>
            {
                (clicks % MOST_CLICKS) + 1
            }
            _ => 1,
        };
        self.window.last_press = Some((now, at, clicks));
        clicks
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/input/selection.rs"]
mod tests;
