use std::path::PathBuf;

use dot_api::{
    Cancellation, Chat, Credential, Effort, Model, Permissions, Prompt, Reply, ReplyError,
    ReplyEvent, Toolbox,
};

use crate::claude_code::ClaudeCodeChat;
use crate::messages::{Authorization, Hands, MessagesChat};

/// [`Chat`] with Claude for one window, through the Messages API or through Claude Code.
///
/// The way depends on the credential of each reply: an API key or an account's access token
/// goes to the Messages API, which streams thinking summaries along with the answer; a Claude
/// plan goes through Claude Code, which keeps the conversation itself. Every reply uses the same
/// model and the same effort. Images go along with the text, the same way both ways.
///
/// Claude works in the folder of `toolbox` both ways. Through the Messages API it has `toolbox`'s
/// tools (files, commands, file and text search) and Anthropic's web search and web fetch;
/// through Claude Code, Claude Code's own tools, settings, skills, agents and MCP servers, as in
/// the user's terminal. Both ways, what writes, runs a command or leaves the folder goes through
/// the reply's [`Permissions`] first.
pub struct ClaudeChat<T> {
    toolbox: T,
    messages: MessagesChat,
    claude_code: ClaudeCodeChat,
}

impl<T: Toolbox> ClaudeChat<T> {
    /// Creates the conversation, without touching the network, the disk or Claude Code yet.
    ///
    /// `model` writes every reply, thinking as much as `effort` asks. `claude_code` is the path
    /// of Claude Code's `claude` program, `None` when it is not installed. `workspace` is a
    /// folder of Dot's own, where Claude Code runs when `toolbox` has no folder; it is created
    /// at the first reply that needs it. `toolbox` runs Claude's tools through the Messages API
    /// and tells both ways which folder Claude works in.
    #[must_use]
    pub fn new(
        model: Model,
        effort: Effort,
        claude_code: Option<PathBuf>,
        workspace: Option<PathBuf>,
        toolbox: T,
    ) -> Self {
        Self {
            toolbox,
            messages: MessagesChat::new(model, effort),
            claude_code: ClaudeCodeChat::new(model, effort, claude_code, workspace),
        }
    }
}

impl<T: Toolbox> Chat for ClaudeChat<T> {
    fn reply(
        &mut self,
        prompt: Prompt,
        credential: &Credential,
        cancellation: &Cancellation,
        permissions: &dyn Permissions,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        let hands = Hands {
            toolbox: &self.toolbox,
            permissions,
            cancellation,
        };
        let authorization = match credential {
            Credential::ApiKey(key) => Authorization::ApiKey(key),
            Credential::AccessToken(token) => Authorization::Bearer(token),
            Credential::ClaudeCode => {
                return self.claude_code.reply(prompt, &hands, on_event);
            }
        };
        self.messages.reply(prompt, authorization, &hands, on_event)
    }

    fn reset(&mut self) {
        self.messages.reset();
        self.claude_code.reset();
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-model/chat.rs"]
mod tests;
