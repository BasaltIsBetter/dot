use std::time::Instant;

use dot_api::{Hue, Text, Translator};

use crate::app::Parts;
use crate::sessions::identity::NameField;
use crate::sessions::{SessionKey, WindowSession};
use dot_transcript::{Ink, Measures, Palette, SPARK_COLUMNS};

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
    pub(crate) foot: Foot,
    pub(crate) hue: Option<Hue>,
    pub(crate) picking: bool,
    pub(crate) field: Option<NameField>,
    pub(crate) name_tint: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CardLook<'a> {
    pub(crate) project: Option<&'a str>,
    pub(crate) colours: bool,
    pub(crate) picking: Option<SessionKey>,
    pub(crate) renaming: Option<&'a NameField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Foot {
    pub(crate) rate: Option<u64>,
    pub(crate) rates: [u16; SPARK_COLUMNS],
}

impl Foot {
    pub(crate) const EMPTY: Self = Self {
        rate: None,
        rates: [0; SPARK_COLUMNS],
    };
}

impl Card {
    pub(crate) fn of<P: Parts>(
        opened: &WindowSession<P>,
        here: bool,
        look: &CardLook<'_>,
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
        let identity = &opened.identity;
        let heat = identity.name_heat(now);
        let identity_rgb = if look.colours {
            identity.hue.rgb()
        } else {
            Hue::Orange.rgb()
        };
        Self {
            name: shown_name(opened, look.project, translator),
            here,
            state,
            foot: conversation
                .measures(now)
                .map_or(Foot::EMPTY, |measures: Measures| Foot {
                    rate: measures.rate,
                    rates: measures.spark.rates,
                }),
            hue: look.colours.then_some(identity.hue),
            picking: look.picking == Some(opened.key),
            field: look
                .renaming
                .filter(|field| field.key == opened.key)
                .cloned(),
            name_tint: (heat > 0)
                .then(|| Palette { live: identity_rgb }.shade(Ink::Full, heat, u8::MAX)),
        }
    }
}

pub(crate) fn shown_name<P: Parts>(
    opened: &WindowSession<P>,
    project: Option<&str>,
    translator: &impl Translator,
) -> String {
    opened
        .identity
        .name
        .clone()
        .unwrap_or_else(|| name_of(opened.number, project, translator))
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
