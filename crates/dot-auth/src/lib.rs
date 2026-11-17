//! Connection of Dot to Claude: an Anthropic API key, or the user's Claude plan through Claude
//! Code.
//!
//! [`AnthropicConnector`] implements the [`dot_api::Connector`] contract. The binary builds it
//! once, with the path that [`locate_claude_code`] returns, and hands it to the interface.
//!
//! An API key is checked with one free request to Anthropic (the list of models) and then kept
//! in the macOS keychain; it never touches the disk in clear. Dot never sees the Claude plan's
//! credentials: Claude Code signs in through Anthropic's own flow and keeps them itself. The
//! choice of the Claude plan holds no secret: it lives in a file of Dot's settings, so that it
//! never makes macOS ask for keychain access.

mod anthropic;
mod claude_code;
mod connector;
mod store;

pub use crate::claude_code::locate_claude_code;
pub use crate::connector::AnthropicConnector;
