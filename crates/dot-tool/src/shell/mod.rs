mod output;
mod process;

use std::hash::BuildHasher;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use dot_api::{Cancellation, Detail, ToolOutput};

use crate::shell::output::Heard;
use crate::shell::process::{Ending, Lingering, Ran, execute};

const TIMEOUT: Duration = Duration::from_mins(10);
const SENTINEL_PREFIX: &str = "__DOT_FOLDER_";
const SENTINEL_SUFFIX: &str = "__";
const GO_HOME: &str = "cd >/dev/null 2>&1";
const NO_OUTPUT: &str = "(no output)";

pub(crate) struct Shell {
    folder: Option<PathBuf>,
    timeout: Duration,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
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
            timeout,
            state: Mutex::new(State::default()),
        }
    }

    pub(crate) fn run(&self, command: &str, cancellation: &Cancellation) -> ToolOutput {
        let mut state = self.state();
        state.lingering.retain(|lingering| !lingering.is_finished());
        let sentinel = state.sentinel.get_or_insert_with(sentinel).clone();
        let start = state
            .current
            .clone()
            .filter(|current| current.is_dir())
            .or_else(|| self.folder.clone());
        let script = script(command, &sentinel, start.is_none());

        let ran = match execute(&script, start.as_deref(), self.timeout, cancellation) {
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
        state.lingering.extend(lingering);
        let Heard { text, folder } = capture.heard(&sentinel);
        if let Some(folder) = folder.filter(|folder| folder.is_dir()) {
            state.current = Some(folder);
        }
        drop(state);
        answer(ending, text, self.timeout)
    }

    pub(crate) fn restart(&self) -> ToolOutput {
        let mut state = self.state();
        state.current = None;
        for lingering in state.lingering.drain(..) {
            lingering.stop();
        }
        let start = self.folder.as_deref().map_or_else(
            || "the home folder".to_owned(),
            |folder| folder.display().to_string(),
        );
        ToolOutput::done(
            format!("The shell starts afresh, in {start}."),
            Detail::Nothing,
        )
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        for lingering in self.state().lingering.drain(..) {
            lingering.stop();
        }
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

fn sentinel() -> String {
    let random = std::hash::RandomState::new();
    let first = random.hash_one(std::process::id());
    let second = random.hash_one(first);
    format!("{SENTINEL_PREFIX}{first:016x}{second:016x}{SENTINEL_SUFFIX}")
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/shell/mod.rs"]
mod tests;
