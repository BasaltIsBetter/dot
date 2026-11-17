#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ink {
    Full,
    Secondary,
    Receding,
    Faint,
    Ground,
    Live,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) ink: Ink,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) indent: u16,
    pub(crate) spans: Vec<Span>,
}

impl Ink {
    pub(crate) const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Full => [0xff, 0xff, 0xff],
            Self::Secondary => [0xa3, 0xa3, 0xa3],
            Self::Receding => [0x66, 0x66, 0x66],
            Self::Faint => [0x38, 0x38, 0x38],
            Self::Ground => [0x1a, 0x1a, 0x1a],
            Self::Live => [0xff, 0x5a, 0x1f],
        }
    }
}

impl Span {
    pub(crate) fn new(text: impl Into<String>, ink: Ink) -> Self {
        Self {
            text: text.into(),
            ink,
        }
    }
}

impl Line {
    pub(crate) const fn new(indent: u16, spans: Vec<Span>) -> Self {
        Self { indent, spans }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/line.rs"]
mod tests;
