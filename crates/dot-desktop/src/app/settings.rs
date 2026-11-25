use std::time::{Duration, Instant};

use dot_api::{Hue, LiveColor, SessionsColumn, Settings, Text, Translator};
use dot_transcript::{Glide, Motion, Palette, opacity};
use winit::keyboard::Key;

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Fit, Layout, Target};
use crate::conversation::{Colours, Stage};
use crate::event::Keystroke;
use crate::settings::{
    self, Actions, Frame, Gesture, Look, Outcome, Setting, SettingsNews, Waiting,
};
use crate::shell::pasted_text;
use crate::shell::translate_menu;
use crate::sign_in::LONGEST_KEY;
use crate::status_bar::Searching;

const OPEN_KEY: &str = ",";
const PASTE_KEY: &str = "v";
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));
const SAVED_TRACE: Motion = Motion::landing(Duration::from_millis(450));
const READING_GLIDE: Motion = Motion::landing(Duration::from_millis(300));

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn settings_news(&mut self, news: &SettingsNews, now: Instant) {
        if let SettingsNews::Unsaved(_) = news {
            let notice = self.translator.translate(Text::SettingsUnsaved);
            self.sessions.active_mut().conversation.notice(&notice, now);
        }
    }

    pub(super) fn toggles_settings(keystroke: &Keystroke) -> bool {
        keystroke.modifiers.super_key()
            && matches!(&keystroke.key, Key::Character(character) if character == OPEN_KEY)
    }

    pub(super) fn open_settings(&mut self, focus: Option<Setting>, by_wheel: bool, now: Instant) {
        if self.screen != Screen::Chat || self.editing.is_open() {
            return;
        }
        self.settings_screen.open(focus, by_wheel, now);
    }

    pub(super) fn close_settings(&mut self, now: Instant) {
        let before = self.settings;
        let outcome = self.settings_screen.close(now);
        self.follow(before, outcome, now);
    }

    pub(super) fn settings_key(&mut self, keystroke: &Keystroke, now: Instant) {
        if Self::toggles_settings(keystroke) {
            self.close_settings(now);
            return;
        }
        let pasting = keystroke.modifiers.super_key()
            && matches!(&keystroke.key, Key::Character(character) if character == PASTE_KEY);
        let gesture = if pasting && self.settings_screen.is_typing_key() {
            pasted_text(LONGEST_KEY).map(Gesture::Type)
        } else {
            settings::gesture(keystroke)
        };
        let Some(gesture) = gesture else {
            return;
        };
        let before = self.settings;
        let defaults = *self.keeper.defaults();
        let outcome = self.settings_screen.gesture(
            gesture,
            (&mut self.settings, &defaults),
            self.translator,
            now,
        );
        self.follow(before, outcome, now);
    }

    pub(super) fn settings_wheel(&mut self, pixels: f32, now: Instant) {
        self.settings_screen.wheel(pixels, self.translator, now);
    }

    pub(super) fn settings_press(&mut self, at: [f32; 2], layout: &Layout, now: Instant) {
        let before = self.settings;
        let defaults = *self.keeper.defaults();
        let frame = settings_frame(layout);
        let actions = self.actions();
        let look = Look {
            settings: &before,
            defaults: &defaults,
            session: self.identity_hue(),
            translator: self.translator,
            hover: None,
            changed: None,
            actions: &actions,
            waiting: Waiting::Nothing,
        };
        let outcome =
            self.settings_screen
                .press(at, (&look, &mut self.settings), (layout, &frame), now);
        self.follow(before, outcome, now);
    }

    pub(super) fn strip_target(&mut self, target: Target, now: Instant) {
        match target {
            Target::Model => self.open_settings(Some(Setting::Model), false, now),
            Target::Effort => self.open_settings(Some(Setting::Effort), false, now),
            Target::Settings if self.settings_screen.is_open() => self.close_settings(now),
            Target::Settings => self.open_settings(None, false, now),
        }
    }

    pub(super) fn fit(&self, now: Instant) -> Fit {
        Fit {
            widest: self.settings.column.characters(),
            rail: self.settings.sessions_column == SessionsColumn::Auto,
            reading: self.reading.position(now),
        }
    }

    pub(super) fn identity_hue(&self) -> Hue {
        if self.settings.session_colors.is_on() {
            self.sessions.active().identity.hue
        } else {
            Hue::Orange
        }
    }

    pub(super) fn colours(&self) -> Colours {
        Colours {
            tool: self
                .settings
                .tool_colors
                .is_on()
                .then(|| self.identity_hue().rgb()),
            states: self.settings.state_colors.is_on(),
        }
    }

    pub(super) fn palette(&self) -> Palette {
        let hue = match self.settings.live_color {
            LiveColor::OfTheSession => self.identity_hue(),
            LiveColor::Fixed(hue) => hue,
        };
        Palette { live: hue.rgb() }
    }

    pub(super) fn settings_look<'s>(
        &'s self,
        (defaults, actions): (&'s Settings, &'s Actions),
    ) -> Look<'s, P::Translator> {
        Look {
            settings: &self.settings,
            defaults,
            session: self.identity_hue(),
            translator: self.translator,
            hover: self.pointer(),
            changed: None,
            actions,
            waiting: Waiting::Nothing,
        }
    }

    pub(super) fn searching(&self, now: Instant) -> Searching {
        let saved_heat = self
            .settings_screen
            .changed_at()
            .and_then(|at| SAVED_TRACE.eased(now.saturating_duration_since(at)))
            .map_or(0, |eased| opacity(1.0 - eased));
        Searching {
            query: self.settings_screen.query().to_owned(),
            matching: self.settings_screen.matching(self.translator),
            saved_heat,
        }
    }

    pub(super) fn tuned_heats(&self, now: Instant) -> [u8; 2] {
        [self.model_tuned_at, self.effort_tuned_at].map(|tuned_at| {
            tuned_at
                .and_then(|at| COOLING.eased(now.saturating_duration_since(at)))
                .map_or(0, |eased| opacity(1.0 - eased))
        })
    }

    pub(super) fn settings_are_moving(&self, now: Instant) -> bool {
        self.settings_screen.is_moving(now)
            || self.reading.is_moving(now)
            || [self.model_tuned_at, self.effort_tuned_at]
                .into_iter()
                .flatten()
                .any(|at| COOLING.lasts(now.saturating_duration_since(at)))
    }

    fn follow(&mut self, before: Settings, outcome: Outcome, now: Instant) {
        match outcome {
            Outcome::Nothing => {}
            Outcome::Changed(_) => self.apply(&before, now),
            Outcome::Closed(changed) => {
                if changed.contains(&Setting::Model) {
                    self.model_tuned_at = Some(now);
                }
                if changed.contains(&Setting::Effort) {
                    self.effort_tuned_at = Some(now);
                }
            }
            Outcome::Act(setting) => self.act(setting, now),
            Outcome::KeyTyped(key) => self.try_key(key),
            Outcome::Refused(text) => {
                let notice = self.translator.translate(text);
                self.sessions.active_mut().conversation.notice(&notice, now);
            }
        }
    }

    fn apply(&mut self, before: &Settings, now: Instant) {
        let settings = self.settings;
        if settings.tuning != before.tuning {
            self.sessions.tune(settings.tuning);
        }
        if settings.permissions != before.permissions {
            self.sessions.permit(settings.permissions);
        }
        if settings.language != before.language {
            self.translator.use_language(settings.language);
            translate_menu(self.translator);
        }
        if settings.reading_line != before.reading_line {
            self.reading.toward(settings.reading_line.share(), now);
        }
        self.keeper.keep(settings);
    }
}

pub(super) fn settings_frame(layout: &Layout) -> Frame {
    let stage = Stage::of(layout);
    Frame::of(layout, stage.centre, stage.floor)
}

pub(super) fn reading_glide(settings: &Settings) -> Glide {
    Glide::at_rest(settings.reading_line.share(), READING_GLIDE)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/settings.rs"]
mod tests;
