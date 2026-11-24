use std::path::PathBuf;

use dot_api::{
    Cancellation, Chat, Compactor, Credential, Instructions, Permissions, Prompt, Reading, Reply,
    ReplyError, ReplyEvent, Summaries, Toolbox, Tuning,
};
use dot_process::LoginPath;

use crate::claude_code::ClaudeCodeChat;
use crate::messages::{Authorization, Hands, MessagesChat};

/// [`Chat`] with Claude for one window, through the Messages API or through Claude Code.
///
/// The way depends on the credential of each reply: an API key or an account's access token
/// goes to the Messages API, which streams thinking summaries along with the answer; a Claude
/// plan goes through Claude Code, which keeps the conversation itself. Every reply uses the
/// model, the effort and the summaries of the latest [`Tuning`]. Images go along with the text,
/// the same way both ways.
///
/// Claude works in the folder of `toolbox` both ways. Through the Messages API it has `toolbox`'s
/// tools (files, commands, file and text search) and Anthropic's web search and web fetch;
/// through Claude Code, Claude Code's own tools, settings, skills, agents and MCP servers, as in
/// the user's terminal. Both ways, what writes, runs a command or leaves the folder goes through
/// the reply's [`Permissions`] first.
///
/// What a command prints is shortened once before Claude reads it, both ways, because every
/// token of an output is read again with each later request: through the Messages API by the
/// [`Compactor`], along with what Dot's own searches find; through Claude Code by Dot's stand-in
/// for its shell.
pub struct ClaudeChat<T, C> {
    toolbox: T,
    compactor: C,
    messages: MessagesChat,
    claude_code: ClaudeCodeChat,
}

impl<T: Toolbox, C: Compactor> ClaudeChat<T, C> {
    /// Creates the conversation, without touching the network, the disk or Claude Code yet.
    ///
    /// `tuning` says which model writes the replies, how much it thinks first and when the
    /// conversation is summarized, until [`Chat::tune`] changes it. `claude_code` is the path
    /// of Claude Code's `claude` program, `None` when it is not installed. `workspace` is a
    /// folder of Dot's own, where Claude Code runs when `toolbox` has no folder; it is created
    /// at the first reply that needs it. `shell` is the path of Dot's stand-in for the shell of
    /// Claude Code's commands, which shortens what they print; without it they print as they
    /// are. `toolbox` runs Claude's tools through the Messages API
    /// and tells both ways which folder Claude works in. `compactor` shortens what Claude reads
    /// from commands and searches.
    #[must_use]
    pub fn new(
        tuning: Tuning,
        claude_code: Option<PathBuf>,
        workspace: Option<PathBuf>,
        shell: Option<PathBuf>,
        toolbox: T,
        compactor: C,
    ) -> Self {
        Self {
            toolbox,
            compactor,
            messages: MessagesChat::new(tuning.model, tuning.effort),
            claude_code: ClaudeCodeChat::new(
                tuning.model,
                tuning.effort,
                claude_code,
                workspace,
                shell,
            )
            .summarizing(tuning.summaries),
        }
    }

    /// Sets the most Dot may do about summaries through Claude Code, whatever the [`Tuning`]
    /// asks: [`Summaries::LeftToClaude`] when the user set them in Claude Code's own settings,
    /// [`Summaries::WhenTheyPay`] unless this is called.
    ///
    /// With [`Summaries::WhenTheyPay`] tuned and allowed, Dot hands Claude Code, at each
    /// prompt, the size beyond which it summarizes the conversation. Dot works it out from what
    /// the conversation has shown so far: what Claude read with each request, what a summary
    /// leaves and has the next request write, and the prices of the model. Nothing is handed
    /// over during the first ten requests of a conversation, nor a size below Claude Code's
    /// minimum of 100,000 tokens.
    #[must_use]
    pub fn summaries_allowed(mut self, allowed: Summaries) -> Self {
        self.claude_code = self.claude_code.allowing(allowed);
        self
    }

    /// Chooses who reads the project through Claude Code when Claude only has to find or
    /// understand something; Claude itself unless this is called.
    ///
    /// With [`Reading::ByACheaperAgent`], Dot hands Claude Code one more agent for this
    /// conversation, `dot-reader`, which reads on a smaller model and reports, and tells Claude
    /// when to use it. The agents the user installed stay as they are.
    #[must_use]
    pub fn reading(mut self, reading: Reading) -> Self {
        self.claude_code = self.claude_code.reading(reading);
        self
    }

    /// Chooses how the project's `CLAUDE.md` reaches Claude through Claude Code; loaded by
    /// Claude Code unless this is called.
    ///
    /// With [`Instructions::HandedByDot`], Dot reads the `CLAUDE.md` of the folder Claude works
    /// in at each prompt, hands it over word for word in the system prompt, keeps Claude Code
    /// from loading that one file again, and has each sub-agent told to read it. The file is
    /// left to Claude Code when it is larger than 256 KiB, is not plain UTF-8 text, is a
    /// symbolic link, imports another file with `@`, or sits at a path that a pattern or a
    /// shell would misread. Every other instruction file is loaded by Claude Code as before.
    #[must_use]
    pub fn instructions(mut self, instructions: Instructions) -> Self {
        self.claude_code = self.claude_code.instructions(instructions);
        self
    }

    /// Has Claude Code, and the commands it runs, find programs where the user's login shell
    /// finds them, as in a terminal; in Dot's own `PATH` unless this is called.
    ///
    /// The shell runs before the first launch of Claude Code that needs it, unless the caller
    /// has read `login_path` already. Commands through the Messages API run in `toolbox`, which
    /// is given its own.
    #[must_use]
    pub fn login_path(mut self, login_path: LoginPath) -> Self {
        self.claude_code = self.claude_code.login_path(login_path);
        self
    }
}

impl<T: Toolbox, C: Compactor> Chat for ClaudeChat<T, C> {
    fn reply(
        &mut self,
        prompt: Prompt,
        credential: &Credential,
        cancellation: &Cancellation,
        permissions: &dyn Permissions,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        let mut hands = Hands {
            toolbox: &mut self.toolbox,
            permissions,
            cancellation,
            compactor: &self.compactor,
        };
        let authorization = match credential {
            Credential::ApiKey(key) => Authorization::ApiKey(key),
            Credential::AccessToken(token) => Authorization::Bearer(token),
            Credential::ClaudeCode => {
                return self.claude_code.reply(prompt, &hands, on_event);
            }
        };
        self.messages
            .reply(prompt, authorization, &mut hands, on_event)
    }

    fn reset(&mut self) {
        self.messages.reset();
        self.claude_code.reset();
    }

    fn tune(&mut self, tuning: Tuning) {
        self.messages.tune(tuning.model, tuning.effort);
        self.claude_code.tune(tuning);
    }

    fn settle(&mut self, cancellation: &Cancellation) {
        self.claude_code.settle(self.toolbox.folder(), cancellation);
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-model/chat.rs"]
mod tests;
