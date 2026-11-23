use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use crate::launch::{command, kill_group, stop};
use crate::relay::output::{Heard, Output};

const LOGIN: &str = "-l";
const INTERACTIVE: &str = "-i";
const SCRIPT: &str = "-c";
const MARK: &str = "__DOT_LOGIN_PATH__";
const LONGEST_WAIT: Duration = Duration::from_secs(5);
const LOOK: Duration = Duration::from_millis(10);
const PIECE: usize = 16 * 1_024;
const TAIL_KEPT: usize = 64 * 1_024;
const MOST_PIECES_AFTER_THE_END: usize = 16;
const NEW_LINE: u8 = b'\n';
const NUL: u8 = 0;
const SHELL_TOOK_TOO_LONG: &str = "the login shell did not print its PATH in time";
const SHELL_PRINTED_NO_PATH: &str = "the login shell printed no PATH";

/// The `PATH` the user's login shell sets up: where the programs the user types in a terminal
/// are found.
///
/// An app opened from the Finder or the Dock gets the bare `PATH` of macOS, without the folders
/// the user's shell configuration adds (Homebrew, `~/.cargo/bin`, nvm…). Dot hands this one to
/// the programs that run the user's commands, so that they find what the terminal finds.
///
/// Clones share what was read: the shell runs once, at the first [`LoginPath::read`].
#[derive(Clone, Debug, Default)]
pub struct LoginPath {
    shell: Option<PathBuf>,
    read: Arc<OnceLock<Option<OsString>>>,
}

impl LoginPath {
    /// Creates the `PATH` that `shell` sets up, without running anything yet.
    ///
    /// `shell` is the user's shell, as `SHELL` names it; a relative path counts as none. The
    /// default, like `None`, reads nothing: programs then keep Dot's own `PATH`.
    #[must_use]
    pub fn new(shell: Option<PathBuf>) -> Self {
        Self {
            shell: shell.filter(|shell| shell.is_absolute()),
            read: Arc::default(),
        }
    }

    /// Returns the `PATH` of the user's login shell, `None` when it could not be read.
    ///
    /// The first call runs the shell as a terminal would (`-l -i -c`), without input, and waits
    /// up to 5 seconds for it; later calls, and calls from other threads meanwhile, wait for that
    /// one and return what it read. What the shell leaves running in its process group is
    /// stopped. A shell that fails or takes longer is logged, and `None` is returned from then
    /// on.
    #[must_use]
    pub fn read(&self) -> Option<&OsStr> {
        self.read
            .get_or_init(|| {
                let Some(shell) = self.shell.as_deref() else {
                    tracing::warn!("commands keep Dot's own PATH: no login shell is known");
                    return None;
                };
                ask(shell, LONGEST_WAIT)
                    .inspect_err(|error| tracing::warn!("commands keep Dot's own PATH: {error}"))
                    .ok()
            })
            .as_deref()
    }
}

fn ask(shell: &Path, longest: Duration) -> io::Result<OsString> {
    let (pipe, writer) = io::pipe()?;
    let mut login = command(shell);
    login
        .args([LOGIN, INTERACTIVE, SCRIPT, &script()])
        .stdout(writer);
    let mut child = login.spawn()?;
    drop(login);

    let printed = printed_by(
        &mut child,
        Output::new(pipe, PIECE),
        Instant::now() + longest,
    );
    kill_group(child.id())?;
    stop(&mut child)?;
    path_between_marks(&printed?)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, SHELL_PRINTED_NO_PATH))
}

fn script() -> String {
    format!("/bin/echo {MARK}; /usr/bin/printenv PATH; /bin/echo {MARK}")
}

fn printed_by(child: &mut Child, mut output: Output, deadline: Instant) -> io::Result<Vec<u8>> {
    let mut printed = Vec::new();
    loop {
        match output.hear(LOOK)? {
            Heard::Piece(piece) => keep_the_tail(&mut printed, piece),
            Heard::Nothing => {}
            Heard::Closed => return Ok(printed),
        }
        if child.try_wait()?.is_some() {
            drain(&mut output, &mut printed)?;
            return Ok(printed);
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, SHELL_TOOK_TOO_LONG));
        }
    }
}

fn drain(output: &mut Output, printed: &mut Vec<u8>) -> io::Result<()> {
    for _ in 0..MOST_PIECES_AFTER_THE_END {
        match output.hear(Duration::ZERO)? {
            Heard::Piece(piece) => keep_the_tail(printed, piece),
            Heard::Nothing | Heard::Closed => return Ok(()),
        }
    }
    Ok(())
}

fn keep_the_tail(printed: &mut Vec<u8>, piece: &[u8]) {
    printed.extend_from_slice(piece);
    if printed.len() > 2 * TAIL_KEPT {
        printed.drain(..printed.len() - TAIL_KEPT);
    }
}

fn path_between_marks(printed: &[u8]) -> Option<OsString> {
    let lines: Vec<&[u8]> = printed.split(|&byte| byte == NEW_LINE).collect();
    let mark = MARK.as_bytes();
    lines
        .windows(3)
        .rev()
        .find(|window| window[0] == mark && window[2] == mark && is_a_path(window[1]))
        .map(|window| OsString::from_vec(window[1].to_vec()))
}

fn is_a_path(line: &[u8]) -> bool {
    !line.is_empty() && line != MARK.as_bytes() && !line.contains(&NUL)
}

#[cfg(test)]
#[path = "../../../tests/dot-process/login_path.rs"]
mod tests;
