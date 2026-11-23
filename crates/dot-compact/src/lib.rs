//! Shorter tool outputs for Claude: what a command or a search prints, told in fewer tokens.
//!
//! [`ShapeCompactor`] implements the [`dot_api::Compactor`] contract. The binary builds one and
//! hands it to the conversation, which passes every output through it once, before Claude reads
//! it. Nothing here reads the disk, the network or the clock: a text goes in, a shorter text
//! comes out, or `None` when there is nothing to gain.
//!
//! What a file reader prints (`cat`, `sed`, `head`...) and what a diff shows are never rewritten.
//! Elsewhere, repeated paths are written once, lists of files are folded with shell braces,
//! indented JSON loses its spaces, terminal colours and progress bars go away, and the progress
//! lines of a build or a test run that passed become a count in square brackets, such as
//! `[+12 lines: build progress]`.
//!
//! By the owner's choice a long output also loses its middle, and a long line its end. Each cut
//! is counted where it was made, and the output then ends with [`dot_api::CUT_OUTPUT_NOTE`], so
//! that Claude can read the whole of it.

mod command;
mod compactor;
mod cut;
mod noise;
mod note;
mod report;
mod shape;

pub use crate::compactor::ShapeCompactor;
pub use crate::cut::Burst;
pub use crate::report::LineStream;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
