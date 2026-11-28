mod prompts;
mod record;

use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::settings::{Effort, Hue, Model};

pub use self::prompts::PromptHistory;
pub use self::record::{
    Asked, Carried, Ending, Failure, Happened, ImageLabel, Opening, Record, changed_lines,
};

const KEPT_ID_DIGITS: usize = 16;
const HEXADECIMAL: u32 = 16;

/// Identifier of a session Dot keeps between launches, unique among those it keeps on this
/// machine: 64 bits, written as 16 lowercase hexadecimal digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeptId(u64);

/// What the window shows of a kept session besides its conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSession {
    /// The session's identifier.
    pub id: KeptId,
    /// The name the user gave the session; `None` until renamed.
    pub name: Option<String>,
    /// The session's colour.
    pub hue: Hue,
    /// The folder the session works in; `None` without one.
    pub folder: Option<PathBuf>,
    /// The model that answers in the session; `None` when none was kept, or one Dot no longer
    /// offers.
    pub model: Option<Model>,
    /// How much Claude thinks first in the session; `None` when none was kept.
    pub effort: Option<Effort>,
}

/// A session as Dot kept it at the last launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restored {
    /// What the window shows of it besides its conversation.
    pub session: KeptSession,
    /// Its conversation, first to last, on the session's own clock.
    pub records: Vec<Record<'static>>,
    /// What its [`Chat`](crate::Chat) remembered, for [`Chat::recall`](crate::Chat::recall);
    /// `None` when nothing was kept or it could not be read.
    pub memory: Option<Vec<u8>>,
}

/// The sessions Dot kept at the last launch, in the order of the window's column.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shelf {
    /// The sessions, first to last.
    pub sessions: Vec<Restored>,
    /// The session the window showed; `None` when it is not among them.
    pub shown: Option<KeptId>,
}

/// The sessions of the window, in the order of its column, and the one shown: what Dot keeps for
/// the next launch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arrangement {
    /// The sessions, first to last.
    pub sessions: Vec<KeptSession>,
    /// The session the window shows.
    pub shown: Option<KeptId>,
}

/// Error from keeping sessions or prompts on the disk, with its cause in English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveError(pub String);

/// Where Dot keeps the sessions of its window between two launches.
///
/// Each session has a [`Journal`]: what happened in its conversation and what its
/// [`Chat`](crate::Chat) remembers. The [`Arrangement`] says which sessions are kept: a session
/// it no longer lists is forgotten, its journal with it, at the next [`Archive::restore`].
pub trait Archive: Send + Sync {
    /// The journal of one session.
    type Journal: Journal + 'static;

    /// Returns the sessions kept at the last launch, each with its conversation and its memory,
    /// and forgets on the disk every kept session that is neither in the last [`Arrangement`]
    /// nor among `alive`, the sessions of this launch.
    ///
    /// A journal grown too large keeps only its latest exchanges, what the others spent carried
    /// over. What cannot be read is left out: a damaged record costs that record, a damaged
    /// memory starts the conversation with Claude afresh.
    ///
    /// Reads and writes the disk: call it off the interface thread.
    fn restore(&self, alive: &[KeptId]) -> Shelf;

    /// Returns the journal of the session `id`, without touching the disk: nothing is written
    /// until the journal is.
    fn journal(&self, id: KeptId) -> Self::Journal;

    /// Keeps `arrangement` for the next launch, in place of the one kept before.
    ///
    /// Writes the disk: call it off the interface thread.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the arrangement cannot be written.
    fn arrange(&self, arrangement: &Arrangement) -> Result<(), ArchiveError>;
}

/// What one session keeps of its conversation between two launches: the [`Record`]s that show
/// it again, and what its [`Chat`](crate::Chat) remembers.
///
/// Every method writes the disk: call them off the interface thread.
pub trait Journal: Send {
    /// Adds `record` after the others. It may wait in memory until [`Journal::flush`].
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the journal cannot be written.
    fn write(&mut self, record: &Record<'_>) -> Result<(), ArchiveError>;

    /// Writes to the disk the records that wait in memory.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the journal cannot be written.
    fn flush(&mut self) -> Result<(), ArchiveError>;

    /// Keeps what `chat` writes, the memory of the session's [`Chat`](crate::Chat), in place of
    /// the one kept before; the one before stays whole until the new one is.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the memory cannot be written, `chat`'s error included.
    fn keep_memory(
        &mut self,
        chat: &dyn Fn(&mut dyn Write) -> io::Result<()>,
    ) -> Result<(), ArchiveError>;
}

impl KeptId {
    /// Creates the identifier `value`.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the identifier written as 16 lowercase hexadecimal digits, or `None` when `text`
    /// is anything else.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_api::KeptId;
    ///
    /// let id = KeptId::new(0x00a1_b2c3_d4e5_f607);
    ///
    /// assert_eq!(id.to_string(), "00a1b2c3d4e5f607");
    /// assert_eq!(KeptId::parse("00a1b2c3d4e5f607"), Some(id));
    /// assert_eq!(KeptId::parse("00A1B2C3D4E5F607"), None);
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let well_shaped = text.len() == KEPT_ID_DIGITS
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !well_shaped {
            return None;
        }
        u64::from_str_radix(text, HEXADECIMAL).ok().map(Self)
    }
}

impl fmt::Display for KeptId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:016x}", self.0)
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ArchiveError {}

impl From<io::Error> for ArchiveError {
    fn from(error: io::Error) -> Self {
        Self(error.to_string())
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/archive/mod.rs"]
mod tests;
