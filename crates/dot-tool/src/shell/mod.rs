mod output;
mod process;

use std::hash::BuildHasher;
use std::path::PathBuf;
use std::time::Duration;

use dot_api::{Cancellation, Detail, ToolOutput};
use dot_process::LoginPath;

use crate::shell::output::Heard;
use crate::shell::process::{Ending, Lingering, Ran, execute};

const TIMEOUT: Duration = Duration::from_mins(10);
const SENTINEL_PREFIX: &str = "__DOT_FOLDER_";
const SENTINEL_SUFFIX: &str = "__";
const GO_HOME: &str = "cd >/dev/null 2>&1";
const NO_OUTPUT: &str = "(no output)";

pub(crate) struct Shell {
    folder: Option<PathBuf>,
    login_path: LoginPath,
    timeout: Duration,
    current: Option<PathBuf>,
    sentinel: Option<String>,
    lingering: Vec<Lingering>,
}

impl Shell {
    pub(crate) fn new(folder: Option<PathBuf>) -> Self {
        Self::with_timeout(folder, TIMEOUT)
    }

    pub(crate) fn with_timeout(folder: Option<PathBuf>, timeout: Duration) -> Self {
        Self {
            folder,
            login_path: LoginPath::default(),
            timeout,
            current: None,
            sentinel: None,
            lingering: Vec::new(),
        }
    }

    pub(crate) fn login_path(mut self, login_path: LoginPath) -> Self {
        self.login_path = login_path;
        self
    }

    pub(crate) fn run(
        &mut self,
        command: &str,
        cancellation: &Cancellation,
        on_output: &mut dyn FnMut(&[u8]),
    ) -> ToolOutput {
        self.lingering.retain(|lingering| !lingering.is_finished());
        let sentinel = self.sentinel.get_or_insert_with(sentinel);
        let start = self
            .current
            .clone()
            .filter(|current| current.is_dir())
            .or_else(|| self.folder.clone());
        let script = script(command, sentinel, start.is_none());

        let path = self.login_path.read();
        let limits = (self.timeout, cancellation);
        let mut shown = |piece: &[u8]| on_output(before_the_sentinel(piece));
        let ran = match execute(&script, start.as_deref(), path, limits, &mut shown) {
            Ok(ran) => ran,
            Err(error) => {
                return ToolOutput::failed(format!("The command could not start: {error}."));
            }
        };
        let Ran {
            ending,
            capture,
            lingering,
        } = ran;
        self.lingering.extend(lingering);
        let Heard { text, folder } = capture.heard(sentinel);
        if let Some(folder) = folder.filter(|folder| folder.is_dir()) {
            self.current = Some(folder);
        }
        answer(ending, text, self.timeout)
    }

    pub(crate) fn restart(&mut self) -> ToolOutput {
        self.current = None;
        self.stop_lingering();
        let start = self.folder.as_deref().map_or_else(
            || "the home folder".to_owned(),
            |folder| folder.display().to_string(),
        );
        ToolOutput::done(
            format!("The shell starts afresh, in {start}."),
            Detail::Nothing,
        )
    }

    fn stop_lingering(&mut self) {
        for lingering in self.lingering.drain(..) {
            lingering.stop();
        }
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.stop_lingering();
    }
}

fn script(command: &str, sentinel: &str, go_home: bool) -> String {
    let home = if go_home { GO_HOME } else { "" };
    format!(
        "exec 2>&1\ntrap 'printf \"\\n%s%s\\n\" \"{sentinel}\" \"$PWD\"' EXIT\n{home}\n{command}\n"
    )
}

fn answer(ending: Ending, text: String, timeout: Duration) -> ToolOutput {
    let text = if text.is_empty() {
        NO_OUTPUT.to_owned()
    } else {
        text
    };
    match ending {
        Ending::Exited(0) => ToolOutput::done(text, Detail::Exit(0)),
        Ending::Exited(code) => {
            ToolOutput::done(format!("{text}\n[exit status {code}]"), Detail::Exit(code))
        }
        Ending::TimedOut => ToolOutput::failed(format!(
            "The command ran longer than {} seconds and was stopped. Its output so far:\n{text}",
            timeout.as_secs()
        )),
        Ending::Stopped => ToolOutput::stopped(),
    }
}

fn before_the_sentinel(piece: &[u8]) -> &[u8] {
    let prefix = SENTINEL_PREFIX.as_bytes();
    let found = piece
        .windows(prefix.len())
        .position(|window| window == prefix);
    found.map_or(piece, |at| &piece[..at])
}

fn sentinel() -> String {
    let random = std::hash::RandomState::new();
    let first = random.hash_one(std::process::id());
    let second = random.hash_one(first);
    format!("{SENTINEL_PREFIX}{first:016x}{second:016x}{SENTINEL_SUFFIX}")
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/shell/mod.rs"]
mod tests;
