/// Language Dot displays its texts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// English, the reference language and the default one.
    English,
    /// French.
    French,
    /// Spanish.
    Spanish,
    /// Brazilian Portuguese.
    Portuguese,
}

impl Language {
    /// Every available language, in the order the settings offer them.
    pub const ALL: [Self; 4] = [Self::English, Self::French, Self::Spanish, Self::Portuguese];

    /// Returns the language's ISO 639-1 code: `en`, `fr`, `es`, `pt`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
            Self::Spanish => "es",
            Self::Portuguese => "pt",
        }
    }

    /// Returns the language's name in the language itself, as the user picks it: `english`,
    /// `français`, `español`, `português`.
    #[must_use]
    pub const fn own_name(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::French => "français",
            Self::Spanish => "español",
            Self::Portuguese => "português",
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
