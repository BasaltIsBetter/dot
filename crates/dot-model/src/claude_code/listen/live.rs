use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use dot_api::{AgentEvent, Compactor, ReplyEvent, ToolKind, ToolUse};
use dot_process::LiveOutputs;

use crate::live::LiveTail;

const LIVE_FOLDER: &str = "live";
const HALF: u32 = 32;

pub(crate) struct LiveCommands<'c> {
    compactor: &'c dyn Compactor,
    running: Vec<Running>,
    connections: Vec<Connection>,
}

struct Running {
    id: String,
    command: String,
    shown: bool,
}

struct Connection {
    number: u64,
    tool: Option<String>,
    shown: bool,
    tail: LiveTail,
}

impl<'c> LiveCommands<'c> {
    pub(crate) fn new(compactor: &'c dyn Compactor) -> Self {
        Self {
            compactor,
            running: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub(crate) fn notice(&mut self, event: &ReplyEvent) {
        match event {
            ReplyEvent::ToolStarted { id, tool } => self.start(id, tool, true),
            ReplyEvent::Agent {
                event: AgentEvent::ToolStarted { id, tool },
                ..
            } => self.start(id, tool, false),
            ReplyEvent::ToolEnded { id, .. }
            | ReplyEvent::Agent {
                event: AgentEvent::ToolEnded { id, .. },
                ..
            } => self.running.retain(|running| running.id != *id),
            _ => {}
        }
    }

    pub(crate) fn heard(
        &mut self,
        connection: u64,
        command: &str,
        bytes: &[u8],
    ) -> Option<ReplyEvent> {
        if bytes.is_empty() {
            self.connections.retain(|open| open.number != connection);
            return None;
        }
        let found = self
            .connections
            .iter()
            .position(|open| open.number == connection);
        let index = found.unwrap_or_else(|| {
            let (tool, shown) = self.tool_for(command);
            self.connections.push(Connection {
                number: connection,
                tool,
                shown,
                tail: LiveTail::default(),
            });
            self.connections.len() - 1
        });
        let open = &mut self.connections[index];
        open.tail.push(bytes);
        let id = open.tool.as_ref().filter(|_| open.shown)?;
        let still_running = self.running.iter().any(|running| running.id == *id);
        still_running.then(|| ReplyEvent::ToolOutput {
            id: id.clone(),
            lines: open.tail.lines(self.compactor),
        })
    }

    fn start(&mut self, id: &str, tool: &ToolUse, shown: bool) {
        if tool.kind == ToolKind::Run {
            self.running.push(Running {
                id: id.to_owned(),
                command: tool.subject.clone(),
                shown,
            });
        }
    }

    fn tool_for(&self, command: &str) -> (Option<String>, bool) {
        let unwatched: Vec<&Running> = self
            .running
            .iter()
            .filter(|running| {
                !self
                    .connections
                    .iter()
                    .any(|open| open.tool.as_ref() == Some(&running.id))
            })
            .collect();
        let found = unwatched
            .iter()
            .find(|running| running.command == command)
            .or_else(|| unwatched.first().filter(|_| unwatched.len() == 1));
        found.map_or((None, false), |running| {
            (Some(running.id.clone()), running.shown)
        })
    }
}

pub(crate) fn live_outputs(workspace: Option<&Path>) -> Option<LiveOutputs> {
    let folder: PathBuf = workspace?.join(LIVE_FOLDER);
    let random = std::hash::RandomState::new().hash_one(std::process::id()) >> HALF;
    let name = format!("{}-{random:08x}", std::process::id());
    match LiveOutputs::open(&folder, &name) {
        Ok(outputs) => Some(outputs),
        Err(error) => {
            tracing::warn!(%error, "what commands print is not shown as it comes");
            None
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/listen/live.rs"]
mod tests;
