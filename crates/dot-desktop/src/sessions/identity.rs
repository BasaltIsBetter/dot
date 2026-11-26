use std::time::{Duration, Instant};

use dot_api::Hue;
use dot_transcript::{Motion, opacity};

use crate::sessions::SessionKey;

const LONGEST_NAME: usize = 24;
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));
const MULTIPLIER: u64 = 0x2545_f491_4f6c_dd1d;
const NONZERO: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) hue: Hue,
    pub(crate) name: Option<String>,
    pub(crate) renamed: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dice {
    state: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NameField {
    pub(crate) key: SessionKey,
    pub(crate) text: String,
    pub(crate) caret: usize,
    pub(crate) whole: bool,
}

impl Identity {
    pub(crate) const fn new(hue: Hue) -> Self {
        Self {
            hue,
            name: None,
            renamed: None,
        }
    }

    pub(crate) fn rename(&mut self, typed: &str, shown: &str, now: Instant) {
        let typed = typed.trim();
        if typed.is_empty() || typed == shown {
            return;
        }
        self.name = Some(typed.to_owned());
        self.renamed = Some(now);
    }

    pub(crate) fn name_heat(&self, now: Instant) -> u8 {
        self.renamed
            .and_then(|at| COOLING.eased(now.saturating_duration_since(at)))
            .map_or(0, |eased| opacity(1.0 - eased))
    }

    pub(crate) fn is_cooling(&self, now: Instant) -> bool {
        self.renamed
            .is_some_and(|at| COOLING.lasts(now.saturating_duration_since(at)))
    }
}

impl Dice {
    pub(crate) const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { NONZERO } else { seed },
        }
    }

    pub(crate) fn roll(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(MULTIPLIER)
    }
}

pub(crate) fn fresh_hue(taken: &[Hue], roll: u64) -> Hue {
    let free: Vec<Hue> = Hue::ALL
        .into_iter()
        .filter(|hue| !taken.contains(hue))
        .collect();
    let choices = if free.is_empty() {
        &Hue::ALL[..]
    } else {
        &free[..]
    };
    let count = u64::try_from(choices.len()).unwrap_or(u64::MAX);
    let picked = usize::try_from(roll % count).unwrap_or(0);
    choices[picked]
}

impl NameField {
    pub(crate) fn of(key: SessionKey, shown: &str) -> Self {
        let text: String = shown.chars().take(LONGEST_NAME).collect();
        Self {
            key,
            caret: text.chars().count(),
            text,
            whole: true,
        }
    }

    pub(crate) fn type_text(&mut self, typed: &str) {
        if self.whole {
            self.text.clear();
            self.caret = 0;
            self.whole = false;
        }
        for character in typed.chars().filter(|character| !character.is_control()) {
            if self.text.chars().count() >= LONGEST_NAME {
                return;
            }
            let at = self.byte_at(self.caret);
            self.text.insert(at, character);
            self.caret += 1;
        }
    }

    pub(crate) fn erase(&mut self) {
        if self.whole {
            self.text.clear();
            self.caret = 0;
            self.whole = false;
            return;
        }
        let Some(before) = self.caret.checked_sub(1) else {
            return;
        };
        let at = self.byte_at(before);
        self.text.remove(at);
        self.caret = before;
    }

    pub(crate) fn left(&mut self) {
        if self.whole {
            self.whole = false;
            self.caret = 0;
            return;
        }
        self.caret = self.caret.saturating_sub(1);
    }

    pub(crate) fn right(&mut self) {
        self.whole = false;
        self.caret = (self.caret + 1).min(self.text.chars().count());
    }

    fn byte_at(&self, characters: usize) -> usize {
        self.text
            .char_indices()
            .nth(characters)
            .map_or(self.text.len(), |(at, _)| at)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sessions/identity.rs"]
mod tests;
