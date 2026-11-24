use dot_api::Language;

const FRENCH_TEXTS: &str = include_str!("../locales/fr.ftl");
const ENGLISH_TEXTS: &str = include_str!("../locales/en.ftl");
const SUBTAG_SEPARATORS: [char; 2] = ['-', '_'];
const DEFAULT: Language = Language::French;

/// Returns the first available language among the user's preferred ones, in their order, or
/// French if none is available.
///
/// Each preference is a language tag such as `fr-FR`, `en_US` or `zh-Hans-CN`: only the
/// language, before the first `-` or `_`, counts, case-insensitively.
///
/// # Examples
///
/// ```
/// use dot_api::Language;
/// use dot_i18n::negotiate;
///
/// assert_eq!(negotiate(["de-DE", "en-US", "fr-FR"]), Language::English);
/// assert_eq!(negotiate(["de-DE"]), Language::French);
/// ```
#[must_use]
pub fn negotiate<I>(preferred: I) -> Language
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    preferred
        .into_iter()
        .find_map(|tag| from_tag(tag.as_ref()))
        .unwrap_or(DEFAULT)
}

pub(crate) const fn texts(language: Language) -> &'static str {
    match language {
        Language::French => FRENCH_TEXTS,
        Language::English => ENGLISH_TEXTS,
    }
}

fn from_tag(tag: &str) -> Option<Language> {
    let primary_subtag = tag.split(SUBTAG_SEPARATORS).next()?;
    Language::with_code(primary_subtag)
}

#[cfg(test)]
#[path = "../../../tests/dot-i18n/language.rs"]
mod tests;
