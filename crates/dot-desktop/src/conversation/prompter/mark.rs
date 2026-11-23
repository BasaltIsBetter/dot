use dot_transcript::{Ink, MARKER, NOTE_MARKER};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Draft,
    Note,
}

impl Mark {
    pub(super) const fn text(self) -> &'static str {
        match self {
            Self::Draft => MARKER,
            Self::Note => NOTE_MARKER,
        }
    }

    pub(super) const fn ink(self) -> Ink {
        match self {
            Self::Draft => Ink::Receding,
            Self::Note => Ink::Live,
        }
    }

    pub(super) const fn caret_ink(self) -> Ink {
        match self {
            Self::Draft => Ink::Full,
            Self::Note => Ink::Live,
        }
    }

    pub(super) const fn hint_ink(self) -> Ink {
        match self {
            Self::Draft => Ink::Receding,
            Self::Note => Ink::Secondary,
        }
    }
}
