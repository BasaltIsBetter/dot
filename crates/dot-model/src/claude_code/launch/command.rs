use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

use dot_api::{COMPACTION_VARIABLE, Effort, OutputCompaction, Reading};
use dot_process::LIVE_OUTPUT_VARIABLE;

use crate::claude_code::launch::Handed;
use crate::tool::{CLAUDE_CODE_NOTE, READER_AGENT, READER_NOTE};

const FIXED_ARGUMENTS: [&str; 12] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
    "--permission-mode",
    "manual",
    "--permission-prompt-tool",
    "stdio",
    "--effort",
];
const MODEL: &str = "--model";
const NOTE_OPTION: &str = "--append-system-prompt";
const RESUME: &str = "--resume";
const SUMMARY_WINDOW: &str = "--autocompact";
const AGENTS: &str = "--agents";
const SETTINGS: &str = "--settings";
const SHELL_PREFIX: &str = "CLAUDE_CODE_SHELL_PREFIX";
const PATH_VARIABLE: &str = "PATH";
const PLAIN_IN_A_PATH: &[u8] = b"/._-";
const SESSION_ID_LENGTH: usize = 36;
const SESSION_ID_DASH: u8 = b'-';

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionId(String);

impl SessionId {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let well_shaped = text.len() == SESSION_ID_LENGTH
            && !text.starts_with(char::from(SESSION_ID_DASH))
            && text
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == SESSION_ID_DASH);
        well_shaped.then(|| Self(text.to_owned()))
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Options<'a> {
    pub(crate) session: Option<&'a SessionId>,
    pub(crate) shell: Option<&'a Path>,
    pub(crate) compaction: OutputCompaction,
    pub(crate) live: Option<&'a Path>,
    pub(crate) path: Option<&'a OsStr>,
    pub(crate) summary_window: Option<u64>,
    pub(crate) reading: Reading,
    pub(crate) instructions: Option<&'a Handed>,
}

pub(crate) fn command(
    program: &Path,
    folder: &Path,
    model: &str,
    effort: Effort,
    options: Options<'_>,
) -> Command {
    let mut command = dot_process::command(program);
    command
        .args(FIXED_ARGUMENTS)
        .arg(effort.api_name())
        .args([MODEL, model, NOTE_OPTION]);
    let about_reading = match options.reading {
        Reading::ByClaude => "",
        Reading::ByACheaperAgent => READER_NOTE,
    };
    let handed = options
        .instructions
        .map_or("", |handed| handed.note.as_str());
    command.arg(format!("{CLAUDE_CODE_NOTE}{about_reading}{handed}"));
    if options.reading == Reading::ByACheaperAgent {
        command.args([AGENTS, READER_AGENT]);
    }
    if let Some(handed) = options.instructions {
        command.args([SETTINGS, &handed.settings]);
    }
    if let Some(SessionId(session)) = options.session {
        command.args([RESUME, session]);
    }
    if let Some(tokens) = options.summary_window {
        command.args([SUMMARY_WINDOW, &tokens.to_string()]);
    }
    if let Some(shell) = options.shell.and_then(plain) {
        command
            .env(SHELL_PREFIX, shell)
            .env(COMPACTION_VARIABLE, options.compaction.to_variable());
    }
    if let Some(live) = options.live {
        command.env(LIVE_OUTPUT_VARIABLE, live);
    }
    if let Some(path) = options.path {
        command.env(PATH_VARIABLE, path);
    }
    command
        .current_dir(folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn plain(path: &Path) -> Option<&str> {
    let written = path.to_str().filter(|_| path.is_absolute())?;
    let is_plain = written
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || PLAIN_IN_A_PATH.contains(&byte));
    if !is_plain {
        tracing::warn!(
            "command outputs are not shortened: the path of Dot's shell stand-in would not survive a shell"
        );
    }
    is_plain.then_some(written)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/command.rs"]
mod tests;
