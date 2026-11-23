use std::time::{Duration, Instant};

use crate::measure::{Measures, Meter};
use crate::transcript::{Glyph, Tool};

/// An agent Claude handed a task to, as Dot follows it from the call that starts it to the
/// report it hands back.
#[derive(Clone, Debug)]
pub struct Agent {
    /// What kind of agent it is, such as `Explore`; empty until the agent says.
    pub kind: String,
    /// The task, in Claude's words.
    pub mission: String,
    /// When the call that hands the task over started.
    pub started: Instant,
    /// When that call ended, once it has.
    pub ended: Option<Instant>,
    /// The tools the agent called, in order.
    pub actions: Vec<Action>,
    /// What the agent said along the way, in order.
    pub said: Vec<String>,
    /// The report the agent handed back, once it has.
    pub report: Option<String>,
    meter: Meter,
    counted: u64,
}

/// A tool an agent called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    /// The identifier of the agent's call.
    pub id: String,
    /// What the tool does and on what.
    pub tool: Tool,
    /// What it measured once it ended; `None` while it runs.
    pub result: Option<String>,
}

/// Every agent of a conversation, in the order Claude started them.
#[derive(Clone, Debug, Default)]
pub struct Agents {
    agents: Vec<(String, Agent)>,
}

impl Agents {
    /// Starts following the agent that the call `id` hands `mission` to, at `now`.
    pub fn start(&mut self, id: String, mission: String, now: Instant) {
        let agent = Agent {
            kind: String::new(),
            mission,
            started: now,
            ended: None,
            actions: Vec::new(),
            said: Vec::new(),
            report: None,
            meter: Meter::new(now),
            counted: 0,
        };
        self.agents.push((id, agent));
    }

    /// Names the kind of the agent of `id`.
    pub fn name(&mut self, id: &str, kind: String) {
        if let Some(agent) = self.get_mut(id) {
            agent.kind = kind;
        }
    }

    /// Counts `tokens` the agent of `id` wrote at `now`, and returns how many of them were not
    /// counted yet: none for an unknown agent.
    pub fn wrote(&mut self, id: &str, tokens: u64, now: Instant) -> u64 {
        let Some(agent) = self.get_mut(id) else {
            return 0;
        };
        agent.counted = agent.counted.saturating_add(tokens);
        agent.meter.hear_tokens(tokens, now);
        tokens
    }

    /// Keeps what the agent of `id` said.
    pub fn said(&mut self, id: &str, text: String) {
        if let Some(agent) = self.get_mut(id) {
            agent.said.push(text);
        }
    }

    /// Adds the tool the agent of `id` calls, under its own `action` identifier.
    pub fn start_action(&mut self, id: &str, action: String, tool: Tool) {
        if let Some(agent) = self.get_mut(id) {
            agent.actions.push(Action {
                id: action,
                tool,
                result: None,
            });
        }
    }

    /// Ends the tool `action` of the agent of `id` with its `result`.
    pub fn end_action(&mut self, id: &str, action: &str, result: String) {
        let found = self
            .get_mut(id)
            .and_then(|agent| agent.actions.iter_mut().find(|called| called.id == action));
        if let Some(called) = found {
            called.result = Some(result);
        }
    }

    /// Keeps the report the agent of `id` handed back, and the `tokens` it wrote over its whole
    /// task; returns how many of them were not counted yet.
    pub fn report(&mut self, id: &str, text: String, tokens: u64, now: Instant) -> u64 {
        let Some(agent) = self.get_mut(id) else {
            return 0;
        };
        agent.report = Some(text);
        let uncounted = tokens.saturating_sub(agent.counted);
        agent.counted = agent.counted.max(tokens);
        agent.meter.hear_tokens(uncounted, now);
        uncounted
    }

    /// Ends the agent of `id` at `now`, with the call that handed its task over.
    pub fn end(&mut self, id: &str, now: Instant) {
        if let Some(agent) = self.get_mut(id) {
            agent.ended.get_or_insert(now);
            agent.meter.end(None, now);
        }
    }

    /// Ends every agent still at work at `now`, when the reply they worked for ends.
    pub fn end_working(&mut self, now: Instant) {
        for (_, agent) in &mut self.agents {
            if agent.ended.is_none() {
                agent.ended = Some(now);
                agent.meter.end(None, now);
            }
        }
    }

    /// Returns the agent of `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.agents
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, agent)| agent)
    }

    /// Returns the agents still at work, the first started first, each with its identifier.
    pub fn working(&self) -> impl Iterator<Item = (&str, &Agent)> {
        self.agents
            .iter()
            .filter(|(_, agent)| agent.ended.is_none())
            .map(|(id, agent)| (id.as_str(), agent))
    }

    /// Forgets every agent.
    pub fn clear(&mut self) {
        self.agents.clear();
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut Agent> {
        self.agents
            .iter_mut()
            .find(|(known, _)| known == id)
            .map(|(_, agent)| agent)
    }
}

impl Agent {
    /// Returns what the agent does now: the last tool it called that still runs, or else the
    /// last one it called.
    #[must_use]
    pub fn current(&self) -> Option<&Action> {
        self.actions
            .iter()
            .rev()
            .find(|action| action.result.is_none())
            .or_else(|| self.actions.last())
    }

    /// Returns the tokens the agent wrote, its rate and its spark at `now`.
    #[must_use]
    pub fn measures(&self, now: Instant) -> Measures {
        self.meter.measures(now)
    }

    /// Returns how long the agent worked: up to its end, or to `now`.
    #[must_use]
    pub fn worked(&self, now: Instant) -> Duration {
        self.ended
            .unwrap_or(now)
            .saturating_duration_since(self.started)
    }

    /// Returns what the agent did most, and how many times: the kind of tool it called most,
    /// the first called on a tie; `None` when it called none.
    #[must_use]
    pub fn mostly(&self) -> Option<(Glyph, u64)> {
        let mut counts: Vec<(Glyph, u64)> = Vec::new();
        for action in &self.actions {
            match counts
                .iter_mut()
                .find(|(glyph, _)| *glyph == action.tool.glyph)
            {
                Some((_, count)) => *count += 1,
                None => counts.push((action.tool.glyph, 1)),
            }
        }
        counts.into_iter().rev().max_by_key(|&(_, count)| count)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/agent/mod.rs"]
mod tests;
