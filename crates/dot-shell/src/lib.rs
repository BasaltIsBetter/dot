//! The program that stands between Claude Code and its shell inside Dot, so that what Claude's
//! commands print is shortened before Claude reads it.
//!
//! Dot names this program in `CLAUDE_CODE_SHELL_PREFIX` when it starts Claude Code. Claude Code
//! then hands it every command line it would have given its shell: Claude's commands, but also
//! the user's hooks, the status line and the start of MCP servers. [`plan`] tells the two apart.
//! Anything that is not a command of Claude, and any command whose output Claude asked for
//! whole, is handed to the real shell, which takes this program's place: nothing is read,
//! nothing is changed. The other commands run in the real shell too, with the same exit status,
//! and [`Shortening`] shortens what they print: whole when they end quickly, line by line when
//! they last. An output that loses something Claude may need is kept whole by [`WholeOutputs`],
//! and its last line says where. A command that leaves
//! something running ends when its shell does: [`leave_passing_on`] takes over what is
//! written after it.

mod plan;
mod script;
mod shortening;
mod whole;

use std::fs::File;
use std::io::{self, ErrorKind, PipeReader};
use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::{ExitStatusExt, parent_id};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

use dot_process::Pace;

use crate::plan::COMMAND_FLAG;
pub use crate::plan::{Plan, plan};
pub use crate::shortening::Shortening;
pub use crate::whole::{
    Kept, MOST_MEBIBYTES_IN_ALL, MOST_MEBIBYTES_KEPT, WholeOutputs, in_own_temporary_folder,
};

/// How often the program looks whether Claude Code, which started it, still runs.
pub const LOOK_AT_THE_PARENT: Duration = Duration::from_millis(200);

/// How long and how much of a command's output is held whole before it is passed on as it comes:
/// two seconds and one mebibyte, with what waits written after a second of silence, or every
/// thirty seconds when the output never pauses.
pub const PACE: Pace = Pace {
    hold: Duration::from_secs(2),
    pause: Duration::from_secs(1),
    wait: Duration::from_secs(30),
    held: 1_024 * 1_024,
};

/// Exit code of a shell that could not be run, as shells report it themselves.
pub const COULD_NOT_RUN: u8 = 127;

/// Exit code of a command whose output could not be passed on, though it may have run.
pub const OUTPUT_LOST: u8 = 1;

const ENDED_BY_A_SIGNAL: i32 = 128;
const COPIER: &str = "/bin/cat";
const REAPER: &str = "/bin/sh";
const REAPER_NAME: &str = "dot-shell-reaper";
const REAP: &str =
    r#"kill -s TERM -- "-$1" 2>/dev/null; sleep 1; kill -s KILL -- "-$1" 2>/dev/null"#;
const FIRST_ORDINARY_GROUP: u32 = 2;
const NOT_A_GROUP_TO_END: &str = "this process group is not one a command leads";

/// Returns the exit code that tells how the command ended: its own, or 128 plus the number of
/// the signal that ended it, as a shell reports it.
#[must_use]
pub fn exit_code(status: ExitStatus) -> u8 {
    let code = status
        .code()
        .or_else(|| status.signal().map(|signal| ENDED_BY_A_SIGNAL + signal));
    code.and_then(|code| u8::try_from(code).ok())
        .unwrap_or(COULD_NOT_RUN)
}

/// Returns whether `first` and `second` are open on the same file, pipe or terminal.
///
/// Two handles whose file cannot be told are taken for different ones.
pub fn share_a_file(first: &impl AsFd, second: &impl AsFd) -> bool {
    let identity = |handle: &dyn AsFd| {
        let file = File::from(handle.as_fd().try_clone_to_owned().ok()?);
        let metadata = file.metadata().ok()?;
        Some((metadata.dev(), metadata.ino()))
    };
    identity(first).is_some_and(|file| Some(file) == identity(second))
}

/// Starts the program that copies to `to`, unchanged, whatever is still written to `output`,
/// and does not wait for it.
///
/// A command may leave something running that goes on writing. Claude Code gives a command a
/// file to write to, so what such a program writes lands there after the command; here the
/// command writes to this program instead, which ends with the command. `cat` takes its place
/// and ends by itself once nothing holds `output` open. It runs in the caller's process group,
/// so whoever stops the command stops it too.
///
/// # Errors
///
/// Returns an error if `cat` cannot start: `output` is then closed, and what still writes to
/// it finds its pipe broken.
pub fn leave_passing_on(output: PipeReader, to: Stdio) -> io::Result<Child> {
    Command::new(COPIER).stdin(output).stdout(to).spawn()
}

/// Returns the exit code that tells why a command did not end as usual: [`COULD_NOT_RUN`] when
/// its shell could not start, [`OUTPUT_LOST`] when what it printed could not be passed on.
#[must_use]
pub fn failure_code(error: &io::Error) -> u8 {
    match error.kind() {
        ErrorKind::NotFound | ErrorKind::PermissionDenied => COULD_NOT_RUN,
        _ => OUTPUT_LOST,
    }
}

/// Looks, on a thread of its own and every `look`, whether the program that started this one
/// is `still_there`, and calls `gone` once it is not.
///
/// Claude Code starts each command in a process group of its own, which Dot cannot see: when
/// Claude Code is stopped, nothing else would end the command. The thread is never waited for:
/// it ends with the process.
pub fn watch_the_parent(
    still_there: impl Fn() -> bool + Send + 'static,
    look: Duration,
    gone: impl FnOnce() + Send + 'static,
) {
    thread::spawn(move || {
        while still_there() {
            thread::sleep(look);
        }
        gone();
    });
}

/// Returns whether the program that started this one is still `parent`.
///
/// When its parent ends, a program is handed to another one: its parent process changes.
#[must_use]
pub fn is_the_parent(parent: u32) -> bool {
    parent_id() == parent
}

/// Starts, in a process group of its own, the program that ends the process group `group`: it
/// asks the group to end, then kills what is left a second later.
///
/// It ends this program too, when this program is in `group`. It is not waited for.
///
/// # Errors
///
/// Returns an error for group 0 or 1, which no command leads, or if `/bin/sh` cannot start.
pub fn reap(group: u32) -> io::Result<Child> {
    if !may_end(group) {
        return Err(io::Error::new(ErrorKind::InvalidInput, NOT_A_GROUP_TO_END));
    }
    dot_process::command(REAPER)
        .args([COMMAND_FLAG, REAP, REAPER_NAME, &group.to_string()])
        .spawn()
}

const fn may_end(group: u32) -> bool {
    group >= FIRST_ORDINARY_GROUP
}

#[cfg(test)]
#[path = "../../../tests/dot-shell/lib.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
