//! The programs Dot starts on the Mac: the shell of Claude's commands, Claude Code, `ant`.
//!
//! [`command`] prepares a program the same way everywhere: Dot's own Anthropic credentials are
//! left behind, and the program leads a process group of its own, so that [`stop`] ends it with
//! everything it started. [`wait`] waits for a program to end, until a deadline or until the
//! task is cancelled, looking at it without holding a thread busy.
//!
//! What a program writes, like a file or a stream, is not trusted: [`read_line`] reads it one
//! line at a time and never holds more of a line than asked, however long the line is.

mod launch;
mod line;
mod wait;

pub use crate::launch::{command, kill_group, stop};
pub use crate::line::{Line, line_count, read_line};
pub use crate::wait::{Waited, wait};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
