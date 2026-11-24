use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::Layout;
use crate::conversation::flow::Conversation;
use crate::conversation::instrument::short;
use crate::conversation::stage::Stage;
use crate::status_bar::Activity;
use dot_transcript::{Glyph, Measures, Tool};

const SHORT_SEPARATOR: &str = " · ";

impl Conversation {
    pub(crate) fn activity(
        &self,
        layout: &Layout,
        now: Instant,
        translator: &impl Translator,
    ) -> Activity {
        if let Some(activity) = self.agent_activity(translator) {
            return activity;
        }
        if !self.transcript.is_live() {
            return Activity::default();
        }
        let mut state = self.state(translator);
        if self.queued.is_some() {
            let queued = translator.translate(Text::StateQueued);
            state = format!("{state}{SHORT_SEPARATOR}{queued}");
        }
        let without_instrument = Stage::of(layout).gutter.is_none();
        let text = match self.instrument.current(now) {
            Some(measures) if without_instrument => {
                format!("{state}{SHORT_SEPARATOR}{}", short(&measures, translator))
            }
            _ => state,
        };
        Activity {
            text: Some(text),
            live: true,
            sessions: None,
            model_heat: 0,
            effort_heat: 0,
            gauge_tint: None,
            gauge_fill: None,
            name: None,
            name_tint: None,
        }
    }

    pub(crate) fn is_live(&self) -> bool {
        self.transcript.is_live()
    }

    pub(crate) const fn is_paused(&self) -> bool {
        self.pause.is_some()
    }

    pub(crate) fn measures(&self, now: Instant) -> Option<Measures> {
        self.instrument.current(now)
    }

    fn state(&self, translator: &impl Translator) -> String {
        if self.pause.is_some() {
            return translator.translate(Text::StatePaused).into_owned();
        }
        let running: Vec<&Tool> = self.transcript.running_tools().collect();
        let agents = running
            .iter()
            .filter(|tool| tool.glyph == Glyph::Agents)
            .count();
        if agents > 0 {
            let agents = u64::try_from(agents).unwrap_or(u64::MAX);
            return translator
                .count(Text::StateWaitingForAgents, agents)
                .into_owned();
        }
        if let Some(tool) = running.last() {
            return translator
                .with(verb_of(tool.glyph), &tool.label)
                .into_owned();
        }
        let doing = if self.writing {
            Text::StateWriting
        } else {
            Text::StateThinking
        };
        translator.translate(doing).into_owned()
    }
}

const fn verb_of(glyph: Glyph) -> Text {
    match glyph {
        Glyph::Read => Text::StateReading,
        Glyph::Search => Text::StateSearching,
        Glyph::Edit => Text::StateEditing,
        Glyph::Create => Text::StateCreating,
        Glyph::Command => Text::StateRunning,
        Glyph::Web => Text::StateBrowsing,
        Glyph::Agents | Glyph::Other => Text::StateUsing,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/activity.rs"]
mod tests;
