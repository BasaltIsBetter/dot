mod ant;
mod claude_code;

use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::Cancellation;

pub(crate) use self::ant::AntCli;
pub use self::ant::locate_ant;
pub(crate) use self::claude_code::ClaudeCodeCli;
pub use self::claude_code::locate_claude_code;

const SYSTEM_DIRECTORIES: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];
const CREDENTIAL_VARIABLES: [&str; 2] = ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"];
const FIRST_POLL: Duration = Duration::from_millis(1);
const LONGEST_POLL: Duration = Duration::from_millis(50);

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
    ) -> Result<String, RunError> {
        let mut child = self.spawn(arguments, Stdio::piped())?;
        let stdout = child.stdout.take();
        succeeded(wait(child, timeout, &Cancellation::new())?)?;

        let mut output = String::new();
        if let Some(stdout) = stdout {
            stdout
                .take(longest)
                .read_to_string(&mut output)
                .map_err(RunError::Wait)?;
        }
        Ok(output)
    }

    fn spawn(&self, arguments: &[&str], stdout: Stdio) -> Result<Child, RunError> {
        self.command(arguments, stdout)
            .spawn()
            .map_err(|error| RunError::Spawn(self.path.display().to_string(), error))
    }

    fn command(&self, arguments: &[&str], stdout: Stdio) -> Command {
        let mut command = Command::new(&self.path);
        for variable in CREDENTIAL_VARIABLES {
            command.env_remove(variable);
        }
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(Stdio::null());
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
    let started = Instant::now();
    let mut pause = FIRST_POLL;
    loop {
        if let Some(status) = child.try_wait().map_err(RunError::Wait)? {
            return Ok(status);
        }
        if cancellation.is_cancelled() {
            stop(&mut child)?;
            return Err(RunError::Cancelled);
        }
        if started.elapsed() >= timeout {
            stop(&mut child)?;
            return Err(RunError::TimedOut(timeout));
        }
        thread::sleep(pause);
        pause = longer(pause);
    }
}

fn longer(pause: Duration) -> Duration {
    (pause * 2).min(LONGEST_POLL)
}

fn stop(child: &mut Child) -> Result<(), RunError> {
    child.kill().map_err(RunError::Wait)?;
    child.wait().map(drop).map_err(RunError::Wait)
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
