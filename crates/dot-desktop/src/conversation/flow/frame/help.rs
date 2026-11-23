use std::time::{Duration, Instant};

use dot_api::{Text, Translator};

use dot_transcript::{Glide, Ink, Line, Motion, Span, columns};

const QUIET_FOR: Duration = Duration::from_secs(5);
const FADE: Motion = Motion::landing(Duration::from_millis(300));
const KEY_INK: Ink = Ink::Full;
const WHAT_INK: Ink = Ink::Secondary;
const KEY_GAP: usize = 2;
const KEYS: [(Text, Text); 6] = [
    (Text::HelpSendKey, Text::HelpSend),
    (Text::HelpNewLineKey, Text::HelpNewLine),
    (Text::HelpPauseKey, Text::HelpPause),
    (Text::HelpStopKey, Text::HelpStop),
    (Text::HelpReviewKey, Text::HelpReview),
    (Text::HelpSwitchKey, Text::HelpSwitch),
];

#[derive(Debug, Default)]
pub(crate) struct Help {
    stirred: Option<Instant>,
    shown: Option<Glide>,
    focused: bool,
}

impl Help {
    pub(crate) const fn stir(&mut self, now: Instant) {
        self.stirred = Some(now);
    }

    pub(crate) fn pace(&mut self, at_rest: bool, focused: bool, now: Instant) {
        self.focused = focused;
        let target = if at_rest && self.is_quiet(now) {
            1.0
        } else {
            0.0
        };
        match self.shown.as_mut() {
            Some(shown) => shown.toward(target, now),
            None => self.shown = Some(Glide::at_rest(target, FADE)),
        }
    }

    pub(crate) fn alpha(&self, now: Instant) -> f64 {
        self.shown
            .map_or(0.0, |shown| f64::from(shown.position(now)))
    }

    pub(crate) fn is_fading(&self, now: Instant) -> bool {
        self.shown.is_some_and(|shown| shown.is_moving(now))
    }

    pub(crate) fn wakes_at(&self, now: Instant) -> Option<Instant> {
        let stirred = self.stirred.filter(|_| self.focused)?;
        let quiet_at = stirred + QUIET_FOR;
        (quiet_at > now).then_some(quiet_at)
    }

    pub(crate) fn lines(&self, translator: &impl Translator, now: Instant) -> Vec<Line> {
        let alpha = self.alpha(now);
        if alpha <= 0.0 {
            return Vec::new();
        }
        let keys = KEYS.map(|(key, what)| {
            (
                translator.translate(key).into_owned(),
                translator.translate(what).into_owned(),
            )
        });
        let widest = keys
            .iter()
            .map(|(key, _)| columns(key))
            .max()
            .unwrap_or_default();
        keys.into_iter()
            .map(|(key, what)| {
                let gap = " ".repeat(widest - columns(&key) + KEY_GAP);
                let spans = vec![Span::new(key + &gap, KEY_INK), Span::new(what, WHAT_INK)];
                Line::new(0, spans).faded(alpha)
            })
            .collect()
    }

    fn is_quiet(&self, now: Instant) -> bool {
        self.stirred
            .is_none_or(|stirred| now.saturating_duration_since(stirred) >= QUIET_FOR)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/frame/help.rs"]
mod tests;
