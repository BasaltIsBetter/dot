use std::time::{Duration, Instant};

use crate::bench::conversation::{end_tool, send, start_tool};
use crate::conversation::Conversation;

/// A reply that runs tools one after the other, as Claude reports them.
pub struct ToolBench {
    conversation: Conversation,
    now: Instant,
    tools: usize,
}

impl ToolBench {
    /// Starts a reply that has already run `tools` tools in a row.
    #[must_use]
    pub fn after(tools: usize) -> Self {
        let now = Instant::now();
        let mut conversation = Conversation::default();
        send(&mut conversation, "look around", now);
        let mut bench = Self {
            conversation,
            now,
            tools: 0,
        };
        for _ in 0..tools {
            bench.run_tool();
        }
        bench
    }

    /// Runs one more tool: started, then ended with what it measured.
    pub fn run_tool(&mut self) {
        self.now += Duration::from_millis(1);
        self.tools += 1;
        let id = self.tools.to_string();
        start_tool(&mut self.conversation, &id, self.now);
        end_tool(&mut self.conversation, id, self.now);
    }
}
