//! Contracts and domain types shared by every Dot crate.

mod cancellation;
mod chat;
mod compaction;
mod connection;
mod file;
mod session;
mod settings;
mod text;
mod tool;

pub use crate::cancellation::{CancelGuard, Cancellation};
pub use crate::chat::{
    AgentEvent, Chat, Image, ImageError, ImageFormat, LIVE_OUTPUT_LINES, Prompt, Reply, ReplyError,
    ReplyEvent, ReplyFailure,
};
pub use crate::compaction::{
    CUT_OUTPUT_NOTE, Compactor, Origin, RAW_OUTPUT_MARK, WHOLE_OUTPUTS_FOLDER,
};
pub use crate::connection::{
    AccessToken, AccessTokenError, ApiKey, ApiKeyError, Cause, ConnectError, ConnectFailure,
    Connection, Connector, Credential, Secret,
};
pub use crate::file::{
    FileChange, Hunk, LARGEST_EDITED, PatchLine, Siblings, Workspace, WorkspaceError,
};
pub use crate::session::{
    Cost, Instructions, Limits, Reading, Session, SessionStatus, Spending, Usage,
};
pub use crate::settings::{
    CaretBlink, Column, Effort, Hue, Language, LiveColor, Model, Preferences, PreferencesError,
    ReadingLine, SessionsColumn, Settings, Summaries, Toggle, Tuning,
};
pub use crate::text::{Text, Translator};
pub use crate::tool::{
    Action, Decision, Detail, Grant, LineRange, Permission, PermissionMode, Permissions, Policy,
    SearchShow, TextSearch, ToolCall, ToolContent, ToolEnd, ToolKind, ToolOutput, ToolUse, Toolbox,
    USER_REFUSED_THE_TOOL, USER_STOPPED_THE_REPLY, Verdict, is_protected,
};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
