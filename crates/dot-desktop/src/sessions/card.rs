use std::time::Instant;

use dot_api::{Text, Translator};

use crate::app::Parts;
use crate::sessions::WindowSession;
use dot_transcript::{Measures, SPARK_COLUMNS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Writing,
    Paused,
    Unread(Instant),
    Idle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Card {
    pub(crate) name: String,
    pub(crate) here: bool,
    pub(crate) state: State,
    pub(crate) foot: Option<Foot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Foot {
    pub(crate) rate: Option<u64>,
    pub(crate) rates: [u16; SPARK_COLUMNS],
}

impl Card {
    pub(crate) fn of<P: Parts>(
        opened: &WindowSession<P>,
        here: bool,
        project: Option<&str>,
        now: Instant,
        translator: &impl Translator,
    ) -> Self {
        let conversation = &opened.conversation;
        let state = if conversation.is_paused() {
            State::Paused
        } else if conversation.is_live() {
            State::Writing
        } else if let Some(since) = opened.unread {
            State::Unread(since)
        } else {
            State::Idle
        };
        Self {
            name: name_of(opened.number, project, translator),
            here,
            state,
            foot: conversation.measures(now).map(|measures: Measures| Foot {
                rate: measures.rate,
                rates: measures.spark.rates,
            }),
        }
    }
}

fn name_of(number: u64, project: Option<&str>, translator: &impl Translator) -> String {
    match project.filter(|project| number == 1 && !project.is_empty()) {
        Some(project) => project.to_owned(),
        None => translator
            .with(Text::SessionName, &number.to_string())
            .into_owned(),
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/sessions/card.rs"]
mod tests;
