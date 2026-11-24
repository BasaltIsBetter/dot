//! Dot's translations: every text, one Fluent file per language (`locales/*.ftl`).
//!
//! The binary picks the [`Language`](dot_api::Language) from the user's settings or the
//! system preferences ([`negotiate`]), then hands a [`FluentTranslator`] to the crates that
//! display text, which may switch it to another language later.
//!
//! # Examples
//!
//! ```
//! use dot_api::{Language, Text, Translator};
//! use dot_i18n::{FluentTranslator, negotiate};
//!
//! let translator = FluentTranslator::new(negotiate(["fr-FR"]));
//! assert_eq!(translator.translate(Text::MenuQuit), "Quitter Dot");
//!
//! translator.use_language(Language::English);
//! assert_eq!(translator.translate(Text::MenuQuit), "Quit Dot");
//! ```

mod language;
mod translator;

pub use crate::language::negotiate;
pub use crate::translator::FluentTranslator;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
