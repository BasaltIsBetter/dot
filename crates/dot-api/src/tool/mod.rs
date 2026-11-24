mod call;
mod permission;
mod policy;
mod report;

use std::path::Path;

use crate::cancellation::Cancellation;

pub use self::call::{LineRange, SearchShow, TextSearch, ToolCall};
pub use self::permission::{Action, Decision, Grant, Permission, PermissionMode, Permissions};
pub use self::policy::{Policy, Verdict, is_protected};
pub use self::report::{
    Detail, ToolContent, ToolEnd, ToolKind, ToolOutput, ToolUse, USER_REFUSED_THE_TOOL,
    USER_STOPPED_THE_REPLY,
};

/// Dot's own tools, which Claude uses on the user's machine, in the session's folder.
pub trait Toolbox: Send + Sync {
    /// Returns the session's folder, where the tools work and commands start; `None` when the
    /// session has none, and every path is then outside it.
    fn folder(&self) -> Option<&Path>;

    /// Returns where `path`, absolute or relative to the folder, lies inside the session's
    /// folder once its links are followed: relative to the folder and spelled as the disk spells
    /// it, `.` for the folder itself; `None` when it leads outside or cannot be followed.
    ///
    /// A path that does not exist yet is judged by the closest folder above it that does. What
    /// is protected is judged on this path, so that neither a link nor the case of a name hides
    /// a `.git` folder.
    fn inside(&self, path: &str) -> Option<String>;

    /// Runs `call` for Claude and returns what Claude reads back.
    ///
    /// Reading inside the folder needs no permission; writing, running a command, or reaching
    /// outside the folder asks `permissions` first, as `tool_id`, and a refusal runs nothing.
    /// Blocks until the tool ends: call it off the interface thread. Taking the toolbox by
    /// `&mut` keeps it to one tool at a time, each command starting where the previous one
    /// ended. Raising `cancellation` ends it at once, a running command included.
    ///
    /// While a command runs, what it prints is handed to `on_output` as it comes, raw, in pieces
    /// that may split a line or a character, a few times a second at most; whatever Claude
    /// reads is still returned whole at the end.
    fn run(
        &mut self,
        call: &ToolCall,
        tool_id: &str,
        permissions: &dyn Permissions,
        cancellation: &Cancellation,
        on_output: &mut dyn FnMut(&[u8]),
    ) -> ToolOutput;
}
