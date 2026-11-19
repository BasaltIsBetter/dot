//! Dot's own tools: what Claude uses on the user's Mac, in the session's folder, when Dot talks
//! to the Messages API itself.
//!
//! [`LocalToolbox`] implements the [`dot_api::Toolbox`] contract. It reads files with their line
//! numbers, images and folders; creates and edits files; runs shell commands in `/bin/bash`;
//! finds files by glob pattern and searches their text with a regular expression, both
//! respecting `.gitignore`.
//!
//! Every path is resolved once, links followed, and judged inside or outside the session's
//! folder. Reading inside is free; writing, running a command, or reaching outside first asks
//! the user through [`dot_api::Permissions`], and a refusal touches nothing. Outputs are bounded
//! so that no file or command can flood Claude's context or Dot's memory.

mod file;
mod place;
mod search;
mod shell;
mod text;
mod toolbox;

pub use crate::toolbox::LocalToolbox;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
