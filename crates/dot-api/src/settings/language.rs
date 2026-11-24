/// Language Dot displays its texts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// French, the reference language.
    French,
    /// English.
    English,
}

impl Language {
    /// Every available language.
    pub const ALL: [Self; 2] = [Self::French, Self::English];

    /// Returns the language's ISO 639-1 code: `fr`, `en`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::French => "fr",
            Self::English => "en",
        }
    }

    /// Returns the language's name in the language itself, as the user picks it: `français`,
    /// `english`.
    #[must_use]
    pub const fn own_name(self) -> &'static str {
        match self {
            Self::French => "français",
            Self::English => "english",
        }
    }

    /// Returns the language whose ISO 639-1 code is `code`, whatever its case, or `None`.
    #[must_use]
    pub fn with_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code().eq_ignore_ascii_case(code))
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/settings/language.rs"]
mod tests;
