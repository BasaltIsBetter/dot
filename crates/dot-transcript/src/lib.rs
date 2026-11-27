//! The conversation as Dot reads it, without a window or a GPU.
//!
//! [`Transcript`] holds what was said: the prompts, Claude's thinking and answers as they
//! stream in, its notices, and the tools it runs. It knows where each entry wraps ([`wrap`],
//! [`TextWidth`]) and what moves on screen: each word of the question giving way to the reply's
//! text, the question coming back above the reply ([`Motion`], [`Glide`]). A [`Reader`] keeps
//! the row being read on a fixed reading line, like a teleprompter, and a [`Meter`] measures
//! each reply: its tokens, its rate and how long it took. [`lines`] turns it into the
//! [`Placed`] lines of a frame, in [`Ink`]s warmed toward the live colour while live.
//!
//! Nothing here draws or reads the clock: every call is given the time, and the caller paints
//! the lines it gets.

mod agent;
mod code;
mod ink;
mod line;
mod measure;
mod motion;
mod reading;
mod selection;
mod transcript;
mod wrap;

pub use crate::agent::{Action, Agent, Agents};
pub use crate::code::{
    CODE_LINES_KEPT, Change, ChangeHunk, ChangeLine, CodeLanguage, CodeWords, Neighbours, code_inks,
};
pub use crate::ink::{Ink, OPAQUE, Palette, opacity};
pub use crate::line::{
    Band, Line, MARKER, NOTE_MARKER, Placed, Span, TEXT_INDENT, TextWidth, WIDEST_COLUMN, columns,
    truncated,
};
pub use crate::measure::{Calibration, Measures, Meter, SPARK_CELLS, SPARK_COLUMNS, Spark};
pub use crate::motion::{Glide, Motion, SETTLE};
pub use crate::reading::{Cadence, Plan, Reader, index, rows, to_f32, words_within};
pub use crate::selection::{RowText, Selection, Spot, paragraph_at, word_at};
pub use crate::transcript::{
    Aside, Glyph, Insertion, Kind, Running, Tool, Transcript, View, heat_after, lines,
};
pub use crate::wrap::{NEW_LINE, cells, normalized, shown, typed, wrap, wrap_characters};

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
