pub(crate) mod run;

use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::Duration;

use dot_api::Cancellation;

use self::run::{Ended, run};

const DOCKER: &str = "docker";
const PODMAN: &str = "podman";
const SCRIPT: &[u8] = b"#!";
const LONGEST_SCRIPT: u64 = 4 * 1_024;
const SYSTEM_DIRECTORIES: [&str; 3] = ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"];
const VERSION: [&str; 3] = ["version", "--format", "{{.Server.Version}}"];
const PODMAN_VERSION: [&str; 3] = ["version", "--format", "{{.Client.Version}}"];
const QUICK: Duration = Duration::from_secs(30);
const BUILD: Duration = Duration::from_mins(30);
const DOCKERFILE: &str = include_str!("../../image/Dockerfile");
const FROM_INPUT: &str = "-";

/// The program that runs the containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    /// Docker, and what answers to its command line.
    Docker,
    /// Podman.
    Podman,
}

/// The user's container engine, through its command line.
#[derive(Clone, Debug)]
pub struct Engine {
    kind: EngineKind,
    program: PathBuf,
}

/// What keeps the engine from doing what it is asked.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Its program cannot start.
    #[error("{} could not start", .0.display())]
    Start(PathBuf, #[source] io::Error),
    /// What it does can no longer be followed.
    #[error("lost track of the container engine")]
    Lost(#[source] io::Error),
    /// It did not answer in time.
    #[error("the container engine did not answer within {} s", .0.as_secs())]
    TimedOut(Duration),
    /// The caller stopped waiting for it.
    #[error("stopped")]
    Cancelled,
    /// It refused, and said why.
    #[error("the container engine refused: {0}")]
    Refused(String),
}

/// Returns the user's container engine, `None` when none is installed: Docker, or else Podman,
/// in the absolute directories of `path` (the `PATH` variable) or where they get installed.
#[must_use]
pub fn locate_engine(path: Option<&OsStr>) -> Option<Engine> {
    let directories: Vec<PathBuf> = path
        .into_iter()
        .flat_map(std::env::split_paths)
        .chain(SYSTEM_DIRECTORIES.map(PathBuf::from))
        .filter(|directory| directory.is_absolute())
        .collect();
    [(EngineKind::Docker, DOCKER), (EngineKind::Podman, PODMAN)]
        .into_iter()
        .find_map(|(kind, name)| {
            let program = directories
                .iter()
                .map(|directory| directory.join(name))
                .find(|program| program.is_file())?;
            let kind = if is_podman(&program) {
                EngineKind::Podman
            } else {
                kind
            };
            Some(Engine::new(kind, program))
        })
}

/// Whether `program` is Podman under whatever name: a link to it, or the script that stands in for
/// Docker on distributions that ship Podman.
fn is_podman(program: &Path) -> bool {
    let linked = fs::canonicalize(program).is_ok_and(|program| program.ends_with(PODMAN));
    let mut start = Vec::new();
    let read =
        File::open(program).and_then(|file| file.take(LONGEST_SCRIPT).read_to_end(&mut start));
    let is_a_script = read.is_ok() && start.starts_with(SCRIPT);
    linked || (is_a_script && String::from_utf8_lossy(&start).contains(PODMAN))
}

impl Engine {
    /// Creates the engine whose program is `program`, without starting it.
    #[must_use]
    pub const fn new(kind: EngineKind, program: PathBuf) -> Self {
        Self { kind, program }
    }

    /// Which engine this is.
    #[must_use]
    pub const fn kind(&self) -> EngineKind {
        self.kind
    }

    /// The path of its program.
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Returns the version of the engine, once it has answered.
    ///
    /// # Errors
    ///
    /// Returns an error if it does not answer: not installed whole, or its service not running.
    pub fn version(&self, cancellation: &Cancellation) -> Result<String, EngineError> {
        let arguments = match self.kind {
            EngineKind::Docker => VERSION,
            EngineKind::Podman => PODMAN_VERSION,
        };
        let version = self.ask(arguments, QUICK, cancellation)?;
        Ok(version.trim().to_owned())
    }

    /// Builds Dot's own image as `tag`, with Claude Code in it.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot build it, or takes more than half an hour.
    pub fn build(&self, tag: &str, cancellation: &Cancellation) -> Result<(), EngineError> {
        let arguments = ["build", "--pull", "--tag", tag, FROM_INPUT].map(OsString::from);
        let mut command = self.command(arguments);
        command.stdin(Stdio::piped());
        let ended = self.run(&mut command, Some(DOCKERFILE), BUILD, cancellation)?;
        ended.printed().map(drop)
    }

    pub(crate) fn ask(
        &self,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<String, EngineError> {
        let mut command = self.command(arguments);
        self.run(&mut command, None, timeout, cancellation)?
            .printed()
    }

    pub(crate) fn command(
        &self,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Command {
        let mut command = dot_process::command(&self.program);
        command
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    pub(crate) fn spawn(&self, command: &mut Command) -> Result<Child, EngineError> {
        command
            .spawn()
            .map_err(|error| EngineError::Start(self.program.clone(), error))
    }

    fn run(
        &self,
        command: &mut Command,
        input: Option<&str>,
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<Ended, EngineError> {
        let mut child = self.spawn(command)?;
        if let (Some(input), Some(to)) = (input, child.stdin.take()) {
            hand_over(input.to_owned(), to);
        }
        run(child, timeout, cancellation)
    }
}

/// Writes `input` from a thread of its own: a program that answers before it has read everything
/// never holds the caller.
pub(crate) fn hand_over(input: String, mut to: ChildStdin) {
    thread::spawn(move || drop(to.write_all(input.as_bytes())));
}

#[cfg(test)]
#[path = "../../../../tests/dot-container/engine/mod.rs"]
pub(crate) mod tests;
