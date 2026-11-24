use std::time::{Duration, Instant};

use dot_api::{Settings, Translator};
use dot_transcript::{Glide, Motion};

use crate::settings::catalog::{ENTRIES, Entry, Setting};

const WHEEL_STEP: f32 = 40.0;
const GESTURE_GAP: Duration = Duration::from_millis(200);
const FIRST_SETTING: Setting = Setting::Model;
const LANDING: Motion = Motion::landing(Duration::from_millis(300));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Up,
    Down,
}

#[derive(Debug)]
pub(crate) struct Reel {
    focus: Setting,
    query: String,
    wheel: f32,
    last_wheel: Option<Instant>,
    opening_gesture: bool,
    changed_since_open: Vec<Setting>,
    pub(crate) shift: Glide,
}

impl Reel {
    pub(crate) fn new() -> Self {
        Self {
            focus: FIRST_SETTING,
            query: String::new(),
            wheel: 0.0,
            last_wheel: None,
            opening_gesture: false,
            changed_since_open: Vec::new(),
            shift: Glide::at_rest(index_of(place_of(FIRST_SETTING)), LANDING),
        }
    }

    pub(crate) fn open(&mut self, focus: Option<Setting>, by_wheel: Option<Instant>, now: Instant) {
        if let Some(setting) = focus {
            self.query.clear();
            self.focus_on(place_of(setting), now);
        }
        self.wheel = 0.0;
        self.changed_since_open.clear();
        self.opening_gesture = by_wheel.is_some();
        self.last_wheel = by_wheel;
    }

    pub(crate) fn close(&mut self) -> Vec<Setting> {
        self.query.clear();
        self.opening_gesture = false;
        std::mem::take(&mut self.changed_since_open)
    }

    pub(crate) const fn focus(&self) -> Setting {
        self.focus
    }

    pub(crate) fn focus_index(&self) -> usize {
        place_of(self.focus)
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn rank(&self) -> (usize, usize) {
        let settings = |entries: &[Entry]| {
            entries
                .iter()
                .filter(|entry| matches!(entry, Entry::Setting(_)))
                .count()
        };
        (
            settings(&ENTRIES[..=self.focus_index()]),
            settings(&ENTRIES),
        )
    }

    pub(crate) fn step(&mut self, step: Step, translator: &impl Translator, now: Instant) {
        let focus = self.focus_index();
        let found = match step {
            Step::Up => (0..focus)
                .rev()
                .find(|&index| self.selectable(index, translator)),
            Step::Down => {
                ((focus + 1)..ENTRIES.len()).find(|&index| self.selectable(index, translator))
            }
        };
        if let Some(index) = found {
            self.focus_on(index, now);
        }
    }

    pub(crate) fn focus_on(&mut self, index: usize, now: Instant) {
        let Some(Entry::Setting(setting)) = ENTRIES.get(index) else {
            return;
        };
        self.focus = *setting;
        self.shift.toward(index_of(index), now);
    }

    pub(crate) fn type_query(&mut self, typed: &str, translator: &impl Translator, now: Instant) {
        self.query.push_str(typed);
        self.refocus(translator, now);
    }

    pub(crate) fn erase_query(&mut self, translator: &impl Translator, now: Instant) -> bool {
        if self.query.pop().is_none() {
            return false;
        }
        self.refocus(translator, now);
        true
    }

    pub(crate) fn clear_query(&mut self) -> bool {
        let had_one = !self.query.is_empty();
        self.query.clear();
        had_one
    }

    pub(crate) fn wheel(&mut self, pixels: f32, translator: &impl Translator, now: Instant) {
        let same_gesture = self
            .last_wheel
            .is_some_and(|last| now.saturating_duration_since(last) < GESTURE_GAP);
        self.last_wheel = Some(now);
        if self.opening_gesture && same_gesture {
            return;
        }
        self.opening_gesture = false;
        self.wheel += pixels;
        while self.wheel.abs() >= WHEEL_STEP {
            let step = if self.wheel > 0.0 {
                Step::Down
            } else {
                Step::Up
            };
            self.step(step, translator, now);
            self.wheel -= WHEEL_STEP.copysign(self.wheel);
        }
    }

    pub(crate) fn changed(&mut self, setting: Setting) {
        if !self.changed_since_open.contains(&setting) {
            self.changed_since_open.push(setting);
        }
    }

    pub(crate) fn matches(&self, index: usize, translator: &impl Translator) -> bool {
        if self.query.is_empty() {
            return true;
        }
        let Some(Entry::Setting(setting)) = ENTRIES.get(index) else {
            return false;
        };
        let section = ENTRIES[..index]
            .iter()
            .rev()
            .find_map(|entry| match entry {
                Entry::Section(text) => Some(*text),
                Entry::Setting(_) => None,
            })
            .map_or_default(|text| translator.translate(text).into_owned());
        let searched = folded(&format!(
            "{} {section}",
            translator.translate(setting.label())
        ));
        searched.contains(&folded(&self.query))
    }

    pub(crate) fn matching(&self, translator: &impl Translator) -> usize {
        (0..ENTRIES.len())
            .filter(|&index| {
                matches!(ENTRIES[index], Entry::Setting(_)) && self.matches(index, translator)
            })
            .count()
    }

    fn selectable(&self, index: usize, translator: &impl Translator) -> bool {
        matches!(ENTRIES[index], Entry::Setting(_)) && self.matches(index, translator)
    }

    fn refocus(&mut self, translator: &impl Translator, now: Instant) {
        if self.matches(self.focus_index(), translator) {
            return;
        }
        if let Some(first) = (0..ENTRIES.len()).find(|&index| self.selectable(index, translator)) {
            self.focus_on(first, now);
        }
    }
}

pub(crate) fn is_changed(setting: Setting, settings: &Settings, defaults: &Settings) -> bool {
    setting.chosen(settings) != setting.chosen(defaults)
}

pub(crate) fn place_of(setting: Setting) -> usize {
    ENTRIES
        .iter()
        .position(|entry| *entry == Entry::Setting(setting))
        .unwrap_or_default()
}

fn index_of(index: usize) -> f32 {
    u16::try_from(index).map_or(f32::MAX, f32::from)
}

fn folded(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|character| match character {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            other => other,
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/settings/reel.rs"]
mod tests;
