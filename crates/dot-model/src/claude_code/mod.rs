mod control;
mod launch;
mod listen;
mod output;
mod tools;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use dot_api::{Effort, Model, Prompt, Reply, ReplyError, ReplyEvent, ReplyFailure, Toolbox};
use serde::Serialize;

use crate::claude_code::launch::{Running, SessionId, command};
use crate::claude_code::listen::{Heard, Listener};
use crate::content::UserMessage;
use crate::messages::Hands;

const EXIT_GRACE: Duration = Duration::from_secs(5);
const NEW_LINE: &[u8] = b"\n";

pub(crate) struct ClaudeCodeChat {
    program: Option<PathBuf>,
    workspace: Option<PathBuf>,
    model: Model,
    effort: Effort,
    session: Mutex<Option<SessionId>>,
}

#[derive(Serialize)]
struct InputLine<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    message: UserMessage<'a>,
}

#[derive(Debug, thiserror::Error)]
enum ClaudeCodeError {
    #[error("Claude Code is not installed")]
    Missing,
    #[error("Dot has no folder to run Claude Code in")]
    NoWorkspace,
    #[error("Claude Code stopped without a result, with {0}")]
    NoResult(std::process::ExitStatus),
}

impl ClaudeCodeChat {
    pub(crate) const fn new(
        model: Model,
        effort: Effort,
        program: Option<PathBuf>,
        workspace: Option<PathBuf>,
    ) -> Self {
        Self {
            program,
            workspace,
            model,
            effort,
            session: Mutex::new(None),
        }
    }

    pub(crate) fn reply<T: Toolbox>(
        &self,
        prompt: &Prompt,
        hands: &Hands<'_, T>,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        let cancellation = hands.cancellation;
        let program = self
            .program
            .as_deref()
            .ok_or_else(|| failed(ClaudeCodeError::Missing))?;
        let folder = self.folder(hands.toolbox.folder())?;

        let mut session = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        let launch = command(
            program,
            folder,
            self.model.id,
            self.effort,
            session.as_ref(),
        );
        let (running, pipes) = Running::start(launch, cancellation).map_err(failed)?;

        let mut stdin = pipes.stdin;
        let handed = hand_over(stdin.as_mut(), prompt);
        let heard = pipes.stdout.map_or_else(
            || Ok(Heard::nothing(hands.toolbox.folder())),
            |stdout| {
                Listener {
                    session: &mut session,
                    stdin: &mut stdin,
                    hands,
                    on_event,
                }
                .listen(stdout)
            },
        );
        drop(stdin);
        let status = running.finish(EXIT_GRACE).map_err(failed)?;
        if cancellation.is_cancelled() {
            return Err(ReplyError::new(ReplyFailure::Stopped));
        }

        let heard = heard.map_err(failed)?;
        let Some(outcome) = heard.outcome else {
            return Err(handed.map_or_else(failed, |()| failed(ClaudeCodeError::NoResult(status))));
        };
        if let Some(failure) = outcome.failure() {
            return Err(ReplyError::new(failure));
        }
        if heard.turn.is_declined() {
            return Err(ReplyError::new(ReplyFailure::Declined));
        }
        Ok(Reply {
            context_tokens: heard.turn.usage().context_tokens(),
            cost: outcome.cost(),
            limits: heard.limits,
        })
    }
}

impl ClaudeCodeChat {
    fn folder<'a>(&'a self, project: Option<&'a Path>) -> Result<&'a Path, ReplyError> {
        if let Some(project) = project {
            return Ok(project);
        }
        let workspace = self
            .workspace
            .as_deref()
            .ok_or_else(|| failed(ClaudeCodeError::NoWorkspace))?;
        fs::create_dir_all(workspace).map_err(failed)?;
        Ok(workspace)
    }
}

fn hand_over(stdin: Option<&mut ChildStdin>, prompt: &Prompt) -> io::Result<()> {
    let Some(stdin) = stdin else {
        return Ok(());
    };
    let line = InputLine {
        kind: "user",
        message: UserMessage::of(prompt),
    };
    let mut written = serde_json::to_vec(&line)?;
    written.extend_from_slice(NEW_LINE);
    stdin.write_all(&written)?;
    stdin.flush()
}

fn failed(cause: impl Into<dot_api::Cause>) -> ReplyError {
    ReplyError::caused_by(ReplyFailure::ClaudeCodeFailed, cause)
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/mod.rs"]
mod tests;
