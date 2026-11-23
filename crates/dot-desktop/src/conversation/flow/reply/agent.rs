use std::time::Instant;

use dot_api::{AgentEvent, ReplyEvent, Text, Translator};

use crate::conversation::flow::Conversation;
use crate::conversation::flow::reply::tool::{result, started};
use crate::status_bar::seconds;
use dot_transcript::{Agent, Glyph};

const AGENT_MARK: &str = "⧉ ";
const SEPARATOR: &str = " · ";

impl Conversation {
    pub(super) fn count_agent(&mut self, event: &ReplyEvent, now: Instant) {
        let ReplyEvent::Agent { id, event } = event else {
            return;
        };
        let uncounted = match event {
            AgentEvent::Wrote { tokens } => self.agents.wrote(id, *tokens, now),
            AgentEvent::Reported { text, tokens } => {
                self.agents.report(id, text.clone(), *tokens, now)
            }
            _ => return,
        };
        self.instrument.hear_tokens(uncounted, now);
    }

    pub(super) fn follow_agent(
        &mut self,
        id: &str,
        event: AgentEvent,
        translator: &impl Translator,
    ) {
        match event {
            AgentEvent::Started { kind } => self.agents.name(id, kind),
            AgentEvent::Said(text) => self.agents.said(id, text),
            AgentEvent::ToolStarted { id: action, tool } => {
                self.agents
                    .start_action(id, action, started(&tool, translator));
            }
            AgentEvent::ToolEnded { id: action, end } => {
                self.agents.end_action(id, &action, result(end, translator));
            }
            AgentEvent::Wrote { .. } | AgentEvent::Reported { .. } => {}
        }
    }
}

pub(in crate::conversation) fn name_of(agent: &Agent) -> &str {
    if agent.kind.is_empty() {
        return &agent.mission;
    }
    &agent.kind
}

pub(in crate::conversation) fn summary(
    agent: &Agent,
    now: Instant,
    translator: &impl Translator,
) -> String {
    let mut summary = name_of(agent).to_owned();
    if let Some((glyph, count)) = agent.mostly() {
        summary.push_str(SEPARATOR);
        summary.push_str(&translator.count(did(glyph), count));
    }
    summary.push_str(SEPARATOR);
    summary.push_str(&seconds(agent.worked(now), translator));
    summary
}

pub(super) fn aside(agent: &Agent, now: Instant, translator: &impl Translator) -> String {
    format!("{AGENT_MARK}{}", summary(agent, now, translator))
}

const fn did(glyph: Glyph) -> Text {
    match glyph {
        Glyph::Read => Text::AgentRead,
        Glyph::Search => Text::AgentSearched,
        Glyph::Edit => Text::AgentEdited,
        Glyph::Create => Text::AgentCreated,
        Glyph::Command => Text::AgentRan,
        Glyph::Web => Text::AgentBrowsed,
        Glyph::Agents => Text::AgentDelegated,
        Glyph::Other => Text::AgentUsed,
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/reply/agent.rs"]
mod tests;
