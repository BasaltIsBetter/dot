use std::process::ExitStatus;

use dot_api::{Cause, ReplyError, ReplyFailure};

const MOST_SAID_SHOWN: usize = 400;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ClaudeCodeError {
    #[error("Claude Code is not installed")]
    Missing,
    #[error("Dot has no folder to run Claude Code in")]
    NoWorkspace,
    #[error("Claude Code stopped without a result, with {0}")]
    NoResult(ExitStatus),
    #[error("Claude Code stopped without a result, with {0}: {1}")]
    NoResultSaying(ExitStatus, String),
}

pub(crate) fn no_result(status: ExitStatus, said: &str) -> ClaudeCodeError {
    let Some(line) = said
        .lines()
        .map(str::trim)
        .rev()
        .find(|line| !line.is_empty())
    else {
        return ClaudeCodeError::NoResult(status);
    };
    let start = line.floor_char_boundary(line.len().saturating_sub(MOST_SAID_SHOWN));
    ClaudeCodeError::NoResultSaying(status, line[start..].to_owned())
}

pub(crate) fn failed(cause: impl Into<Cause>) -> ReplyError {
    ReplyError::caused_by(ReplyFailure::ClaudeCodeFailed, cause)
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/error.rs"]
mod tests;
