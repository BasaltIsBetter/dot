/// One of the eight colours a session can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hue {
    /// `#ff5a1f`.
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
    /// `#a970ff`, Dot's own.
    Violet,
    /// `#ff4fa3`.
    Pink,
}

/// Something the user turns on or off, such as a colour of meaning (Dot is white on black
/// without it) or a way of shortening what Claude reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    /// In use: the colour is shown, the shortening is done.
    On,
    /// Not in use: white instead, the output left as it is.
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

/// How large Dot draws everything: text, margins, the status bar and the settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceSize {
    /// 80 %, the size Dot had before the setting existed.
    Small,
    /// 90 %.
    Medium,
    /// 100 %, Dot's default.
    Large,
    /// 115 %.
    Larger,
    /// 130 %.
    Largest,
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
}

impl Toggle {
    /// Returns whether it is in use.
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

impl InterfaceSize {
    /// Every size, from the smallest to the largest.
    pub const ALL: [Self; 5] = [
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::Larger,
        Self::Largest,
    ];

    /// Returns the size as a percentage of the points of the screen.
    #[must_use]
    pub const fn percent(self) -> u16 {
        match self {
            Self::Small => 80,
            Self::Medium => 90,
            Self::Large => 100,
            Self::Larger => 115,
            Self::Largest => 130,
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
