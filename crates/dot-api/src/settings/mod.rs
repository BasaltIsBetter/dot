mod language;
mod look;
mod outputs;
mod tuning;

use std::error::Error;
use std::fmt;

use crate::text::Text;
use crate::tool::PermissionMode;

pub use self::language::Language;
pub use self::look::{CaretBlink, Column, Hue, InterfaceSize, ReadingLine, SessionsColumn, Toggle};
pub use self::outputs::{COMPACTION_VARIABLE, CutBounds, CutLevel, OutputCompaction};
pub use self::tuning::{Effort, Model, Summaries, Tuning};

/// Everything the user chooses in Dot's settings, for every session of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// How Claude answers, from the next prompt on.
    pub tuning: Tuning,
    /// Whether Claude's actions are allowed at once or put to the user, from the next prompt on.
    pub permissions: PermissionMode,
    /// The colour of the session shown: what changes right now, its card, and the tools and the
    /// context gauge when their colours are on. Kept as the colour of the first session at the
    /// next launch; each new session takes one no other session has.
    pub color: Hue,
    /// `✓` in green, `✗` in red, a stopped reply in amber, and the context gauge in the session's
    /// [`color`](Self::color) while it grows and from 80 % on.
    pub state_colors: Toggle,
    /// The glyph of each tool in the session's [`color`](Self::color).
    pub tool_colors: Toggle,
    /// Language of every text Dot displays.
    pub language: Language,
    /// How large everything is drawn.
    pub size: InterfaceSize,
    /// Most characters on a line of the conversation.
    pub column: Column,
    /// Height of the band where the user writes.
    pub reading_line: ReadingLine,
    /// How the `_` behaves.
    pub caret: CaretBlink,
    /// When the column of session cards shows.
    pub sessions_column: SessionsColumn,
}

/// The system refused to keep the settings, with its own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferencesError(pub String);

/// Where the user's [`Settings`] are kept between two launches of Dot.
pub trait Preferences: Send + Sync {
    /// Returns the settings kept, each one that is missing or cannot be read at its value in
    /// `defaults`.
    ///
    /// Reads the disk: call it at launch, before the window opens, or off the interface thread.
    fn load(&self, defaults: &Settings) -> Settings;

    /// Keeps `settings` for the next launch: only what differs from `defaults`, so that a
    /// setting left at its default follows Dot's default if it changes later.
    ///
    /// Writes the disk: call it off the interface thread.
    ///
    /// # Errors
    ///
    /// Returns a [`PreferencesError`] when the settings cannot be written.
    fn save(&self, settings: &Settings, defaults: &Settings) -> Result<(), PreferencesError>;
}

impl PreferencesError {
    /// Returns what the interface tells the user.
    #[must_use]
    pub const fn text(&self) -> Text {
        Text::SettingsUnsaved
    }
}

impl fmt::Display for PreferencesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.text().id(), self.0)
    }
}

impl Error for PreferencesError {}

#[cfg(test)]
#[path = "../../../../tests/dot-api/settings/mod.rs"]
mod tests;
