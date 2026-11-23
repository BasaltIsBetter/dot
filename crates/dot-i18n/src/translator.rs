use std::borrow::Cow;

use dot_api::{Text, Translator};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::language::Language;

const COUNT: &str = "count";
const VALUE: &str = "value";
const EMBEDDED_FILES_ARE_TESTED: &str =
    "les fichiers de traduction sont embarqués à la compilation et vérifiés par les tests";

/// [`Translator`] that reads texts from the Fluent files embedded in the binary.
///
/// No file is read at runtime: the chosen language is parsed once, at construction.
pub struct FluentTranslator {
    bundle: FluentBundle<FluentResource>,
}

impl FluentTranslator {
    /// Loads the texts of a language.
    ///
    /// # Panics
    ///
    /// Panics if the language's file is invalid. It is embedded at compile time and checked by
    /// this crate's tests: a binary whose tests pass never panics here.
    #[must_use]
    pub fn new(language: Language) -> Self {
        let identifier: LanguageIdentifier =
            language.code().parse().expect(EMBEDDED_FILES_ARE_TESTED);
        let resource = FluentResource::try_new(language.texts().to_owned())
            .map_err(|(_, errors)| errors)
            .expect(EMBEDDED_FILES_ARE_TESTED);

        let mut bundle = FluentBundle::new(vec![identifier]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .expect(EMBEDDED_FILES_ARE_TESTED);
        Self { bundle }
    }
}

impl FluentTranslator {
    fn format(&self, text: Text, arguments: Option<&FluentArgs<'_>>) -> Cow<'_, str> {
        let Some(pattern) = self
            .bundle
            .get_message(text.id())
            .and_then(|message| message.value())
        else {
            return Cow::Borrowed(text.id());
        };

        let mut errors = Vec::new();
        let translated = self.bundle.format_pattern(pattern, arguments, &mut errors);
        debug_assert!(errors.is_empty(), "{}: {errors:?}", text.id());
        translated
    }
}

impl Translator for FluentTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        self.format(text, None)
    }

    fn count(&self, text: Text, count: u64) -> Cow<'_, str> {
        let mut arguments = FluentArgs::new();
        arguments.set(COUNT, count);
        self.format(text, Some(&arguments))
    }

    fn with(&self, text: Text, value: &str) -> Cow<'_, str> {
        let mut arguments = FluentArgs::new();
        arguments.set(VALUE, value);
        self.format(text, Some(&arguments))
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-i18n/translator.rs"]
mod tests;
