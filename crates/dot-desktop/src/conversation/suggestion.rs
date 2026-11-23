use std::time::{Duration, Instant};

use dot_api::{Text, Translator};
use dot_transcript::{Line, Placed, SETTLE, to_f32};

const SUGGESTIONS: [Text; 8] = [
    Text::SuggestionExplainProject,
    Text::SuggestionFixBug,
    Text::SuggestionWriteTests,
    Text::SuggestionReviewChanges,
    Text::SuggestionSimplifyCode,
    Text::SuggestionFindStartup,
    Text::SuggestionSummarizeCommits,
    Text::SuggestionRunTests,
];
const SHOWN_FOR: Duration = Duration::from_secs(4);
const TRAVEL: f32 = 0.7;

#[derive(Debug, Default)]
pub(crate) struct Suggestions {
    first: usize,
    since: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Hint {
    pub(crate) text: String,
    leaving: Option<(String, f32)>,
}

impl Hint {
    pub(crate) const fn still(text: String) -> Self {
        Self {
            text,
            leaving: None,
        }
    }

    pub(crate) fn placed(
        &self,
        top: f32,
        line_height: f32,
        line_of: impl Fn(&str) -> Line,
    ) -> Vec<Placed> {
        let Some((leaving, progress)) = &self.leaving else {
            return vec![Placed {
                top,
                line: line_of(&self.text),
                clip: None,
            }];
        };
        let travel = TRAVEL * line_height;
        vec![
            Placed {
                top: (top - progress * travel).round(),
                line: line_of(leaving).faded(f64::from(1.0 - progress)),
                clip: None,
            },
            Placed {
                top: (top + (1.0 - progress) * travel).round(),
                line: line_of(&self.text).faded(f64::from(*progress)),
                clip: None,
            },
        ]
    }
}

impl Suggestions {
    pub(crate) fn turn(&mut self, now: Instant) {
        self.since.get_or_insert(now);
    }

    pub(crate) fn hold(&mut self, now: Instant) {
        self.first = self.index(now);
        self.since = None;
    }

    pub(crate) fn hint(&self, now: Instant, translator: &impl Translator) -> Hint {
        let index = self.index(now);
        let leaving = self.leaving(now).map(|progress| {
            let previous = SUGGESTIONS[(index + SUGGESTIONS.len() - 1) % SUGGESTIONS.len()];
            (translator.translate(previous).into_owned(), progress)
        });
        Hint {
            text: translator.translate(SUGGESTIONS[index]).into_owned(),
            leaving,
        }
    }

    pub(crate) fn is_rolling(&self, now: Instant) -> bool {
        self.leaving(now).is_some()
    }

    pub(crate) fn next_change(&self, now: Instant) -> Option<Instant> {
        let into_period = into_period(self.since?, now);
        Some(now + SHOWN_FOR.saturating_sub(into_period))
    }

    fn leaving(&self, now: Instant) -> Option<f32> {
        let since = self.since?;
        if self.periods(now) == 0 {
            return None;
        }
        SETTLE.eased(into_period(since, now)).map(to_f32)
    }

    fn index(&self, now: Instant) -> usize {
        (self.first + self.periods(now)) % SUGGESTIONS.len()
    }

    fn periods(&self, now: Instant) -> usize {
        let Some(since) = self.since else {
            return 0;
        };
        let elapsed = now.saturating_duration_since(since);
        usize::try_from(elapsed.as_nanos() / SHOWN_FOR.as_nanos()).unwrap_or(usize::MAX)
    }
}

fn into_period(since: Instant, now: Instant) -> Duration {
    let elapsed = now.saturating_duration_since(since).as_nanos();
    let into_period = elapsed % SHOWN_FOR.as_nanos();
    Duration::from_nanos(u64::try_from(into_period).unwrap_or(u64::MAX))
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/suggestion.rs"]
mod tests;
