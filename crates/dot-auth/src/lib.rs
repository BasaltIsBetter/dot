//! Connection of Dot to Claude: the user's Claude plan through Claude Code, an Anthropic account
//! through `ant`, or an Anthropic API key.
//!
//! [`anthropic_connector`] returns the [`dot_api::Connector`] to Claude. The binary builds it
//! once, with the paths that [`locate_claude_code`] and [`locate_ant`] return, and hands it to
//! the interface.
//!
//! An API key is checked with one free request to Anthropic (the list of models) and then kept
//! in the macOS keychain, or the keyring of a Linux desktop; it never touches the disk in clear. Dot never sees the credentials of
//! a Claude plan or of an account: Claude Code and `ant` sign in through Anthropic's own flow
//! and keep them themselves. `ant` hands out a short-lived access token for each request. The
//! choice of Claude Code or of an account holds no secret: it lives in a file of Dot's settings,
//! so that it never makes macOS ask for keychain access.

mod accounts;
mod anthropic;
mod connector;
mod program;
mod store;

pub use crate::connector::anthropic_connector;
pub use crate::program::{locate_ant, locate_claude_code};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
