mod ant;
mod claude_code;

use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::Cancellation;
use dot_process::Waited;

pub(crate) use self::ant::AntCli;
pub use self::ant::locate_ant;
pub(crate) use self::claude_code::ClaudeCodeCli;
pub use self::claude_code::locate_claude_code;

const SYSTEM_DIRECTORIES: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];
const READER_PANICKED: &str = "reading what the program printed failed";

pub(crate) struct Program {
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RunError {
    #[error("could not run {0}")]
    Spawn(String, #[source] io::Error),
    #[error("lost track of the program")]
    Wait(#[source] io::Error),
    #[error("the program did not finish within {} s", .0.as_secs())]
    TimedOut(Duration),
    #[error("the program was cancelled")]
    Cancelled,
    #[error("the program stopped with {0}")]
    Failed(ExitStatus),
}

impl Program {
    pub(crate) const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn run(
        &self,
        arguments: &[&str],
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<(), RunError> {
        let child = self.spawn(arguments, Stdio::null())?;
        succeeded(wait(child, timeout, cancellation)?)
    }

    pub(crate) fn output(
        &self,
        arguments: &[&str],
        timeout: Duration,
        longest: u64,
        cancellation: &Cancellation,
    ) -> Result<String, RunError> {
        let mut child = self.spawn(arguments, Stdio::piped())?;
        let leader = child.id();
        let stdout = child.stdout.take();
        thread::scope(|scope| {
            let reader = stdout.map(|stdout| scope.spawn(move || read_at_most(stdout, longest)));
            let status = wait(child, timeout, cancellation);
            dot_process::end_groups(&[leader], dot_process::GRACE).map_err(RunError::Wait)?;
            let printed = match reader {
                Some(reader) => reader
                    .join()
                    .unwrap_or_else(|_| Err(io::Error::other(READER_PANICKED)))
                    .map_err(RunError::Wait)?,
                None => String::new(),
            };
            succeeded(status?)?;
            Ok(printed)
        })
    }

    fn spawn(&self, arguments: &[&str], stdout: Stdio) -> Result<Child, RunError> {
        self.command(arguments, stdout)
            .spawn()
            .map_err(|error| RunError::Spawn(self.path.display().to_string(), error))
    }

    fn command(&self, arguments: &[&str], stdout: Stdio) -> Command {
        let mut command = dot_process::command(&self.path);
        command.args(arguments).stdout(stdout);
        command
    }
}

pub(crate) fn locate(
    program: &str,
    home_directories: &[&str],
    home: Option<&Path>,
    path: Option<&OsStr>,
    is_file: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let searched = path.into_iter().flat_map(std::env::split_paths);
    let installed = home.into_iter().flat_map(|home| {
        home_directories
            .iter()
            .map(move |directory| home.join(directory))
    });
    let system = SYSTEM_DIRECTORIES.map(PathBuf::from);

    searched
        .chain(installed)
        .chain(system)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(program))
        .find(|candidate| is_file(candidate))
}

fn wait(
    mut child: Child,
    timeout: Duration,
    cancellation: &Cancellation,
) -> Result<ExitStatus, RunError> {
    let deadline = Instant::now() + timeout;
    let waited =
        dot_process::wait(&mut child, Some(deadline), cancellation).map_err(RunError::Wait)?;
    let failure = match waited {
        Waited::Exited(status) => return Ok(status),
        Waited::TimedOut => RunError::TimedOut(timeout),
        Waited::Cancelled => RunError::Cancelled,
    };
    dot_process::stop(&mut child).map_err(RunError::Wait)?;
    Err(failure)
}

fn read_at_most(mut stdout: impl Read, longest: u64) -> io::Result<String> {
    let mut kept = Vec::new();
    (&mut stdout).take(longest).read_to_end(&mut kept)?;
    io::copy(&mut stdout, &mut io::sink())?;
    String::from_utf8(kept).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn succeeded(status: ExitStatus) -> Result<(), RunError> {
    if !status.success() {
        return Err(RunError::Failed(status));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/dot-auth/program/mod.rs"]
mod tests;
