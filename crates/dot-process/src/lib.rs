//! The programs Dot starts on the Mac: the shell of Claude's commands, Claude Code, `ant`.
//!
//! [`command`] prepares a program the same way everywhere: Dot's own Anthropic credentials are
//! left behind, and the program leads a process group of its own, so that [`stop`] ends it with
//! everything it started: asked to end first, killed after a [`GRACE`]. [`wait`] waits for a
//! program to end, until a deadline or until the task is cancelled, looking at it without
//! holding a thread busy. [`terminate_group`], [`kill_group`] and [`end_groups`] signal groups
//! directly, without starting a program, and never the caller's own group.
//!
//! [`relay`] runs a program in the place of another and passes its output on through a
//! [`Relay`]: whole when the program ends quickly, piece by piece when it lasts. It ends with
//! the program, and hands back the output that something left running still holds open.
//!
//! What a program writes, like a file or a stream, is not trusted: [`read_line`] reads it one
//! line at a time and never holds more of a line than asked, however long the line is.
//!
//! What a command prints can also be watched as it comes: its stand-in shell sends it through a
//! [`LiveOutput`], [`Watched`] by its relay, to the [`LiveOutputs`] Dot listens to.
//!
//! [`LoginPath`] reads, once, the `PATH` the user's login shell sets up, for the programs that
//! run the user's commands: an app opened from the Finder only gets the bare `PATH` of macOS.

#[cfg(target_os = "linux")]
mod ended;
mod launch;
mod line;
mod live;
mod login_path;
mod relay;
mod wait;

pub use crate::launch::{
    GRACE, command, end_groups, group_is_alive, group_led_by, kill_group, own_group, stop,
    terminate_group,
};
pub use crate::line::{Line, line_count, open_without_waiting, read_line};
pub use crate::live::{LIVE_OUTPUT_VARIABLE, LiveOutput, LiveOutputs, LivePiece, Watched};
pub use crate::login_path::LoginPath;
pub use crate::relay::{Errors, Pace, Relay, Relayed, relay};
pub use crate::wait::{Waited, wait};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
