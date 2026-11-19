//! Contracts and domain types shared by every Dot crate.

mod cancellation;
mod chat;
mod connection;
mod session;
mod text;
mod tool;

pub use crate::cancellation::Cancellation;
pub use crate::chat::{
    Chat, Image, ImageError, ImageFormat, Prompt, Reply, ReplyError, ReplyEvent, ReplyFailure,
};
pub use crate::connection::{
    AccessToken, AccessTokenError, ApiKey, ApiKeyError, Cause, ConnectError, ConnectFailure,
    Connection, Connector, Credential,
};
pub use crate::session::{Cost, Effort, Limits, Model, Session, SessionStatus, Usage};
pub use crate::text::{Text, Translator};
pub use crate::tool::{
    Action, Decision, Detail, Grant, LineRange, Permission, PermissionMode, Permissions,
    SearchShow, TextSearch, ToolCall, ToolContent, ToolEnd, ToolKind, ToolOutput, ToolUse, Toolbox,
    USER_REFUSED_THE_TOOL, USER_STOPPED_THE_REPLY,
};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
