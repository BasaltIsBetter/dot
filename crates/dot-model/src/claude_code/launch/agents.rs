use std::path::{Path, PathBuf};

use serde_json::json;

const SERVE: &str = "mcp";
const PROJECT_VARIABLE: &str = "DOT_AGENTS_PROJECT";
const FOLDER_VARIABLE: &str = "DOT_AGENTS_FOLDER";
const EXPANDED_BY_CLAUDE_CODE: &str = "${";

/// The program that calls the agents of containers for Claude, and the folder that describes them.
#[derive(Clone, Debug)]
pub(crate) struct AgentServer {
    pub(crate) program: PathBuf,
    pub(crate) described_in: PathBuf,
}

impl AgentServer {
    /// The MCP configuration that has Claude Code start the program for `folder`; `None` when a
    /// path cannot be written in it.
    ///
    /// The paths travel in variables: with Dot's stand-in for its shell, Claude Code writes the
    /// program and its arguments on one unquoted line. The program lies beside that stand-in, whose
    /// own path has to be plain for Claude Code to be handed it.
    pub(crate) fn configuration(&self, folder: &Path) -> Option<String> {
        let program = as_it_is(&self.program)?;
        let variables = json!({
            PROJECT_VARIABLE: as_it_is(folder)?,
            FOLDER_VARIABLE: as_it_is(&self.described_in)?,
        });
        let server =
            json!({"type": "stdio", "command": program, "args": [SERVE], "env": variables});
        Some(json!({"mcpServers": {"dot-agents": server}}).to_string())
    }
}

fn as_it_is(path: &Path) -> Option<&str> {
    path.to_str()
        .filter(|path| !path.contains(EXPANDED_BY_CLAUDE_CODE))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/agents.rs"]
mod tests;
