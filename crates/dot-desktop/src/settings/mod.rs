mod action;
mod catalog;
mod keeper;
mod reel;
mod view;

use std::time::Instant;

use dot_api::{ApiKey, Settings, Text, Translator};
use dot_transcript::{Motion, SETTLE};
use winit::keyboard::{Key, NamedKey};

use crate::canvas::{Layout, Scene};
use crate::conversation::Stage;
use crate::event::Keystroke;

use self::action::Pending;
use self::catalog::Kind;
use self::reel::{Reel, Step};
use self::view::{Drawn, Hit};

pub(crate) use self::action::Waiting;
pub(crate) use self::catalog::Setting;
pub(crate) use self::keeper::{Keeper, SettingsNews};
pub(crate) use self::view::{Actions, Frame, Look};

const SPACE: &str = " ";
const FADE: Motion = SETTLE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Gesture {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Erase,
    Type(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Nothing,
    Changed(Setting),
    Closed(Vec<Setting>),
    Act(Setting),
    KeyTyped(ApiKey),
    Refused(Text),
}

#[derive(Debug)]
pub(crate) struct SettingsScreen {
    reel: Reel,
    open: bool,
    toggled: Option<Instant>,
    changed: Option<(Setting, Instant)>,
    pending: Pending,
}

impl SettingsScreen {
    pub(crate) fn new() -> Self {
        Self {
            reel: Reel::new(),
            open: false,
            toggled: None,
            changed: None,
            pending: Pending::default(),
        }
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn open(&mut self, focus: Option<Setting>, by_wheel: bool, now: Instant) {
        if self.open {
            return;
        }
        self.open = true;
        self.toggled = Some(now);
        self.pending.clear();
        self.reel.open(focus, by_wheel.then_some(now), now);
    }

    pub(crate) fn close(&mut self, now: Instant) -> Outcome {
        if !self.open {
            return Outcome::Nothing;
        }
        self.open = false;
        self.toggled = Some(now);
        self.pending.clear();
        Outcome::Closed(self.reel.close())
    }

    pub(crate) fn gesture(
        &mut self,
        gesture: Gesture,
        (settings, defaults): (&mut Settings, &Settings),
        translator: &impl Translator,
        now: Instant,
    ) -> Outcome {
        let setting = self.reel.focus();
        if let Some(outcome) = self.pending.gesture(setting, &gesture) {
            return outcome;
        }
        let chosen = setting.chosen(settings);
        match gesture {
            Gesture::Up => self.reel.step(Step::Up, translator, now),
            Gesture::Down => self.reel.step(Step::Down, translator, now),
            Gesture::Left => return self.choose(setting, chosen.saturating_sub(1), settings, now),
            Gesture::Right => return self.choose(setting, chosen + 1, settings, now),
            Gesture::Enter => {
                let next = (chosen + 1) % setting.options();
                return self.choose(setting, next, settings, now);
            }
            Gesture::Escape if self.reel.clear_query() => {}
            Gesture::Escape => return self.close(now),
            Gesture::Erase if self.reel.erase_query(translator, now) => {}
            Gesture::Erase => {
                return self.choose(setting, setting.chosen(defaults), settings, now);
            }
            Gesture::Type(typed) if typed == SPACE || typed.chars().any(char::is_control) => {}
            Gesture::Type(typed) => self.reel.type_query(&typed, translator, now),
        }
        Outcome::Nothing
    }

    pub(crate) fn wheel(&mut self, pixels: f32, translator: &impl Translator, now: Instant) {
        self.pending.clear();
        self.reel.wheel(pixels, translator, now);
    }

    pub(crate) const fn is_typing_key(&self) -> bool {
        self.pending.is_typing_key()
    }

    pub(crate) fn press<T: Translator>(
        &mut self,
        at: [f32; 2],
        (look, settings): (&Look<'_, T>, &mut Settings),
        (layout, frame): (&Layout, &Frame),
        now: Instant,
    ) -> Outcome {
        let drawn = view::entries(&self.reel, look, (layout, frame), now);
        let hit = drawn
            .hits
            .iter()
            .rev()
            .find(|(bounds, _)| view::inside(at, *bounds))
            .map(|(_, hit)| *hit);
        match hit {
            Some(Hit::Choose(setting, _))
                if setting == self.reel.focus() && setting.kind() == Kind::Action =>
            {
                self.pending
                    .gesture(setting, &Gesture::Enter)
                    .unwrap_or(Outcome::Nothing)
            }
            Some(Hit::Choose(setting, option)) if setting == self.reel.focus() => {
                self.choose(setting, option, settings, now)
            }
            Some(Hit::Choose(setting, _)) => {
                self.pending.clear();
                self.reel.focus_on(reel::place_of(setting), now);
                Outcome::Nothing
            }
            Some(Hit::Focus(place)) => {
                if place != reel::place_of(self.reel.focus()) {
                    self.pending.clear();
                }
                self.reel.focus_on(place, now);
                Outcome::Nothing
            }
            None => Outcome::Nothing,
        }
    }

    pub(crate) fn draw<T: Translator>(
        &self,
        scene: &mut Scene,
        look: &Look<'_, T>,
        (layout, frame): (&Layout, &Frame),
        now: Instant,
    ) {
        let shown = self.shown(now);
        if shown <= 0.0 {
            return;
        }
        let look = Look {
            changed: self.changed,
            waiting: self.pending.waiting(),
            ..*look
        };
        let Drawn {
            panes, mut marks, ..
        } = view::entries(&self.reel, &look, (layout, frame), now);
        let band = view::band(layout, frame);
        marks.push(Stage::of(layout).tick(&band));
        let mut drawn = Scene {
            panes,
            grounds: vec![band],
            marks,
            ..Scene::default()
        };
        drawn.panes.push(view::hint(
            self.reel.focus(),
            look.translator,
            (layout, frame),
        ));
        if layout.room().has_gutter() {
            let (panes, marks) = view::instrument(&self.reel, &look, (layout, frame), now);
            drawn.panes.extend(panes);
            drawn.marks.extend(marks);
        }
        if shown < 1.0 {
            drawn.fade_around([0.0, 0.0], dot_transcript::opacity(shown), 1.0);
        }
        scene.panes.extend(drawn.panes);
        scene.grounds.extend(drawn.grounds);
        scene.marks.extend(drawn.marks);
    }

    pub(crate) fn shown(&self, now: Instant) -> f64 {
        let eased = self
            .toggled
            .and_then(|toggled| FADE.eased(now.saturating_duration_since(toggled)))
            .unwrap_or(1.0);
        if self.open { eased } else { 1.0 - eased }
    }

    pub(crate) fn query(&self) -> &str {
        self.reel.query()
    }

    #[cfg(test)]
    pub(crate) const fn focus(&self) -> Setting {
        self.reel.focus()
    }

    pub(crate) fn matching(&self, translator: &impl Translator) -> usize {
        self.reel.matching(translator)
    }

    pub(crate) const fn changed_at(&self) -> Option<Instant> {
        match self.changed {
            Some((_, at)) => Some(at),
            None => None,
        }
    }

    pub(crate) fn is_moving(&self, now: Instant) -> bool {
        let fading = self
            .toggled
            .is_some_and(|toggled| FADE.lasts(now.saturating_duration_since(toggled)));
        fading || self.reel.shift.is_moving(now) || view::is_cooling(self.changed, now)
    }

    fn choose(
        &mut self,
        setting: Setting,
        option: usize,
        settings: &mut Settings,
        now: Instant,
    ) -> Outcome {
        let before = *settings;
        setting.choose(option, settings);
        if *settings == before {
            return Outcome::Nothing;
        }
        self.changed = Some((setting, now));
        self.reel.changed(setting);
        Outcome::Changed(setting)
    }
}

pub(crate) fn gesture(keystroke: &Keystroke) -> Option<Gesture> {
    let modifiers = keystroke.modifiers;
    if modifiers.super_key() || modifiers.control_key() {
        return None;
    }
    let gesture = match &keystroke.key {
        Key::Named(NamedKey::ArrowUp) => Gesture::Up,
        Key::Named(NamedKey::ArrowDown) => Gesture::Down,
        Key::Named(NamedKey::ArrowLeft) => Gesture::Left,
        Key::Named(NamedKey::ArrowRight) => Gesture::Right,
        Key::Named(NamedKey::Enter) => Gesture::Enter,
        Key::Named(NamedKey::Escape) => Gesture::Escape,
        Key::Named(NamedKey::Backspace) => Gesture::Erase,
        _ => Gesture::Type(keystroke.text.as_deref()?.to_owned()),
    };
    Some(gesture)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/settings/mod.rs"]
pub(crate) mod tests;
