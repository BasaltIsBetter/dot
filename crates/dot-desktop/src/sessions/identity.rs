use std::time::{Duration, Instant};

use dot_api::Hue;
use dot_transcript::{Motion, opacity};

use crate::sessions::SessionKey;

const LONGEST_NAME: usize = 24;
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) hue: Hue,
    pub(crate) name: Option<String>,
    pub(crate) renamed: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NameField {
    pub(crate) key: SessionKey,
    pub(crate) text: String,
    pub(crate) caret: usize,
    pub(crate) whole: bool,
}

impl Identity {
    pub(crate) fn new<'a>(taken: impl IntoIterator<Item = &'a Self>, count: usize) -> Self {
        let used: Vec<Hue> = taken.into_iter().map(|identity| identity.hue).collect();
        let hue = Hue::ALL
            .into_iter()
            .find(|hue| !used.contains(hue))
            .unwrap_or(Hue::ALL[count % Hue::ALL.len()]);
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
