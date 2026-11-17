const FRENCH_TEXTS: &str = include_str!("../locales/fr.ftl");
const ENGLISH_TEXTS: &str = include_str!("../locales/en.ftl");
const SUBTAG_SEPARATORS: [char; 2] = ['-', '_'];

/// Language Dot displays its texts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// French, the reference language; Dot picks it when no preferred language is available.
    French,
    /// English.
    English,
}

impl Language {
    /// Every available language.
    pub const ALL: [Self; 2] = [Self::French, Self::English];

    const DEFAULT: Self = Self::French;

    /// Returns the first available language among the user's preferred ones, in their order, or
    /// French if none is available.
    ///
    /// Each preference is a language tag such as `fr-FR`, `en_US` or `zh-Hans-CN`: only the
    /// language, before the first `-` or `_`, counts, case-insensitively.
    ///
    /// # Examples
    ///
    /// ```
    /// use dot_i18n::Language;
    ///
    /// assert_eq!(Language::negotiate(["de-DE", "en-US", "fr-FR"]), Language::English);
    /// assert_eq!(Language::negotiate(["de-DE"]), Language::French);
    /// ```
    #[must_use]
    pub fn negotiate<I>(preferred: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        preferred
            .into_iter()
            .find_map(|tag| Self::from_tag(tag.as_ref()))
            .unwrap_or(Self::DEFAULT)
    }

    /// Returns the language's ISO 639-1 code: `fr`, `en`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::French => "fr",
            Self::English => "en",
        }
    }

    pub(crate) const fn texts(self) -> &'static str {
        match self {
            Self::French => FRENCH_TEXTS,
            Self::English => ENGLISH_TEXTS,
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        let primary_subtag = tag.split(SUBTAG_SEPARATORS).next()?;
        Self::ALL
            .into_iter()
            .find(|language| language.code().eq_ignore_ascii_case(primary_subtag))
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-i18n/language.rs"]
mod tests;
