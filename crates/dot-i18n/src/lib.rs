//! Dot's translations: every text, one Fluent file per language (`locales/*.ftl`).
//!
//! The binary picks the [`Language`] from the system preferences, then hands a
//! [`FluentTranslator`] to the crates that display text.
//!
//! # Examples
//!
//! ```
//! use dot_api::{Text, Translator};
//! use dot_i18n::{FluentTranslator, Language};
//!
//! let translator = FluentTranslator::new(Language::negotiate(["fr-FR"]));
//! assert_eq!(translator.translate(Text::MenuQuit), "Quitter Dot");
//! ```

mod language;
mod translator;

pub use crate::language::Language;
pub use crate::translator::FluentTranslator;

#[cfg(test)]
#[path = "../../../tests/memory_ceiling.rs"]
mod memory_ceiling;
