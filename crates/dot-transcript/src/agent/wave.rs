use std::time::Instant;

use crate::agent::{Agent, Agents};

impl Agents {
    /// Returns the agents of the latest wave, the first started first: the agent Claude started
    /// while none was at work, and every agent started after it.
    ///
    /// Empty without any agent.
    pub fn wave(&self) -> impl ExactSizeIterator<Item = &Agent> {
        self.agents
            .get(self.wave..)
            .unwrap_or_default()
            .iter()
            .map(|(_, agent)| agent)
    }

    /// Returns when the latest wave ended: when the last of its agents ended.
    ///
    /// `None` while one of them is still at work, and without any agent.
    #[must_use]
    pub fn wave_ended(&self) -> Option<Instant> {
        self.wave()
            .try_fold(None, |latest: Option<Instant>, agent| {
                agent.ended.map(|ended| latest.max(Some(ended)))
            })
            .flatten()
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/agent/wave.rs"]
mod tests;
