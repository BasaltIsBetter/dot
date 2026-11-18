const HOTTEST: u32 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Ink {
    Full,
    Secondary,
    Receding,
    Faint,
    Ground,
    Live,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) ink: Ink,
    pub(crate) heat: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
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
            heat: 0,
        }
    }

    pub(crate) fn heated(self, heat: u8) -> Self {
        Self { heat, ..self }
    }

    pub(crate) fn rgb(&self) -> [u8; 3] {
        let settled = self.ink.rgb();
        let live = Ink::Live.rgb();
        std::array::from_fn(|channel| mix(settled[channel], live[channel], self.heat))
    }
}

impl Line {
    pub(crate) const fn new(indent: u16, spans: Vec<Span>) -> Self {
        Self { indent, spans }
    }

    pub(crate) fn has_text(&self) -> bool {
        self.spans.iter().any(|span| !span.text.is_empty())
    }
}

fn mix(settled: u8, live: u8, heat: u8) -> u8 {
    let heat = u32::from(heat);
    let blended =
        (u32::from(settled) * (HOTTEST - heat) + u32::from(live) * heat + HOTTEST / 2) / HOTTEST;
    u8::try_from(blended).unwrap_or(u8::MAX)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/line.rs"]
mod tests;
