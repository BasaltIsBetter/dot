use std::path::{Path, PathBuf};

use serde_json::json;

/// The program that calls the agents of containers for Claude, and the folder that describes them.
#[derive(Clone, Debug)]
pub(crate) struct AgentServer {
    pub(crate) program: PathBuf,
    pub(crate) described_in: PathBuf,
}

impl AgentServer {
    /// The MCP configuration that has Claude Code start the program for `folder`; `None` when a
    /// path cannot be written in it.
    pub(crate) fn configuration(&self, folder: &Path) -> Option<String> {
        let program = self.program.to_str()?;
        let arguments = [
            "--folder",
            folder.to_str()?,
            "--agents",
            self.described_in.to_str()?,
            "mcp",
        ];
        let server = json!({"type": "stdio", "command": program, "args": arguments});
        Some(json!({"mcpServers": {"dot-agents": server}}).to_string())
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/agents.rs"]
mod tests;
