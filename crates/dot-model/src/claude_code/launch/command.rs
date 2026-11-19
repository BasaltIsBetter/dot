use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use dot_api::Effort;

use crate::tool::CLAUDE_CODE_NOTE;

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
const CREDENTIAL_VARIABLES: [&str; 2] = ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"];
const OWN_PROCESS_GROUP: i32 = 0;
const KILL: &str = "/bin/kill";
const KILL_SIGNAL: &str = "-KILL";
const END_OF_OPTIONS: &str = "--";
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

pub(crate) fn command(
    program: &Path,
    folder: &Path,
    model: &str,
    effort: Effort,
    session: Option<&SessionId>,
) -> Command {
    let mut command = Command::new(program);
    command.args(FIXED_ARGUMENTS).arg(effort.api_name()).args([
        MODEL,
        model,
        NOTE_OPTION,
        CLAUDE_CODE_NOTE,
    ]);
    if let Some(SessionId(session)) = session {
        command.args([RESUME, session]);
    }
    for variable in CREDENTIAL_VARIABLES {
        command.env_remove(variable);
    }
    command
        .current_dir(folder)
        .process_group(OWN_PROCESS_GROUP)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

pub(crate) fn kill_group(leader: u32) -> io::Result<ExitStatus> {
    Command::new(KILL)
        .args([KILL_SIGNAL, END_OF_OPTIONS, &format!("-{leader}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/launch/command.rs"]
mod tests;
