//! Claude's replies for Dot: the Messages API over HTTPS with an API key or an Anthropic
//! account, or Claude Code with a Claude plan.
//!
//! [`ClaudeChat`] implements the [`dot_api::Chat`] contract. The binary builds it once per
//! window and hands it to the interface, which asks it for each reply off its own thread.
//!
//! With an API key or an account, Dot speaks to `api.anthropic.com` itself: it keeps the
//! conversation, sends it whole with each prompt, images included, and reads the reply as it
//! streams, thinking summaries included. Claude's tools run through the [`dot_api::Toolbox`] the
//! binary hands over, until Claude has finished. With a Claude plan, Claude Code carries the
//! conversation: Dot starts `claude -p` for each prompt, with the user's own Claude Code tools,
//! settings and plugins, hands it the prompt and its images on its input, reads its stream and
//! answers its permission requests. Both ways, what writes, runs or leaves the folder goes
//! through the reply's [`dot_api::Permissions`] first, and the reply stops at once when it is
//! cancelled.

#[cfg(feature = "bench")]
pub mod bench;
mod chat;
mod claude_code;
mod content;
mod messages;
mod pricing;
mod stream;
mod tool;

pub use crate::chat::ClaudeChat;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
