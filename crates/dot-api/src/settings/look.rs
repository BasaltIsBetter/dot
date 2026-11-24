/// One of the eight colours Dot gives to what changes right now and to each session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hue {
    /// `#ff5a1f`, Dot's own.
    Orange,
    /// `#ff3b3b`.
    Red,
    /// `#ffb020`.
    Amber,
    /// `#3ddc84`.
    Green,
    /// `#22d3ee`.
    Cyan,
    /// `#4d7cff`.
    Blue,
    /// `#a970ff`.
    Violet,
    /// `#ff4fa3`.
    Pink,
}

/// Colour of what changes right now: text that arrives, what the user types, the measures of a
/// reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveColor {
    /// The colour of the session shown; orange while sessions have no colours of their own.
    OfTheSession,
    /// The same colour whatever the session.
    Fixed(Hue),
}

/// A colour of meaning that the user turns on or off: Dot is white on black without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    /// The colour is shown.
    On,
    /// White instead.
    Off,
}

/// Most characters on a line of the conversation; a narrow window holds fewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    /// 64 characters.
    Narrow,
    /// 80 characters.
    Medium,
    /// 96 characters, Dot's default.
    Wide,
}

/// Height of the reading line, the band where the user writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingLine {
    /// Halfway down the window.
    Middle,
    /// A third of the way down.
    Third,
}

/// How the `_` at the end of the prompt and of a reply behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretBlink {
    /// On for 530 ms, off for 530 ms, like an old terminal.
    Blinking,
    /// Always on.
    Steady,
}

/// When the column of session cards shows at the right of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionsColumn {
    /// As soon as the window is wide enough.
    Auto,
    /// Never: the status bar counts the sessions instead.
    Never,
}

impl Hue {
    /// Every hue, in the order the user sees them.
    pub const ALL: [Self; 8] = [
        Self::Orange,
        Self::Red,
        Self::Amber,
        Self::Green,
        Self::Cyan,
        Self::Blue,
        Self::Violet,
        Self::Pink,
    ];

    /// Returns the colour as red, green and blue, from 0 to 255.
    #[must_use]
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Orange => [0xff, 0x5a, 0x1f],
            Self::Red => [0xff, 0x3b, 0x3b],
            Self::Amber => [0xff, 0xb0, 0x20],
            Self::Green => [0x3d, 0xdc, 0x84],
            Self::Cyan => [0x22, 0xd3, 0xee],
            Self::Blue => [0x4d, 0x7c, 0xff],
            Self::Violet => [0xa9, 0x70, 0xff],
            Self::Pink => [0xff, 0x4f, 0xa3],
        }
    }

    /// Returns the first hue of [`Hue::ALL`] that `taken` does not hold, or the one at
    /// `count` modulo eight when all are taken: each new session gets a colour of its own while
    /// one is free.
    #[must_use]
    pub fn first_free(taken: &[Self], count: usize) -> Self {
        Self::ALL
            .into_iter()
            .find(|hue| !taken.contains(hue))
            .unwrap_or(Self::ALL[count % Self::ALL.len()])
    }
}

impl Toggle {
    /// Returns whether the colour is shown.
    #[must_use]
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

impl Column {
    /// Returns the most characters a line holds.
    #[must_use]
    pub const fn characters(self) -> u16 {
        match self {
            Self::Narrow => 64,
            Self::Medium => 80,
            Self::Wide => 96,
        }
    }
}

impl ReadingLine {
    /// Returns how far down the window the line sits, as a share of its height.
    #[must_use]
    pub const fn share(self) -> f32 {
        match self {
            Self::Middle => 0.5,
            Self::Third => 1.0 / 3.0,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/settings/look.rs"]
mod tests;
