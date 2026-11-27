use std::borrow::Cow;
use std::cell::{Cell, OnceCell};

use dot_api::{Language, Text, Translator};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::language::{MACHINE_TERMS, texts};

const COUNT: &str = "count";
const VALUE: &str = "value";
const EMBEDDED_FILES_ARE_TESTED: &str =
    "the translation files are embedded at compile time and checked by the tests";

type Bundle = FluentBundle<FluentResource>;

/// [`Translator`] that reads texts from the Fluent files embedded in the binary.
///
/// No file is read at runtime: a language is parsed the first time it is used, once.
pub struct FluentTranslator {
    language: Cell<Language>,
    bundles: [OnceCell<Bundle>; Language::ALL.len()],
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
        let translator = Self {
            language: Cell::new(language),
            bundles: Default::default(),
        };
        translator.bundle();
        translator
    }

    fn bundle(&self) -> &Bundle {
        let language = self.language.get();
        self.bundles[slot(language)].get_or_init(|| load(language))
    }

    fn format(&self, text: Text, arguments: Option<&FluentArgs<'_>>) -> Cow<'_, str> {
        let bundle = self.bundle();
        let Some(pattern) = bundle
            .get_message(text.id())
            .and_then(|message| message.value())
        else {
            return Cow::Borrowed(text.id());
        };

        let mut errors = Vec::new();
        let translated = bundle.format_pattern(pattern, arguments, &mut errors);
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

    fn use_language(&self, language: Language) {
        self.language.set(language);
    }
}

const fn slot(language: Language) -> usize {
    match language {
        Language::English => 0,
        Language::French => 1,
        Language::Spanish => 2,
        Language::Portuguese => 3,
    }
}

fn load(language: Language) -> Bundle {
    let identifier: LanguageIdentifier = language.code().parse().expect(EMBEDDED_FILES_ARE_TESTED);
    let mut bundle = FluentBundle::new(vec![identifier]);
    bundle.set_use_isolating(false);
    for source in [MACHINE_TERMS, texts(language)] {
        let resource = FluentResource::try_new(source.to_owned())
            .map_err(|(_, errors)| errors)
            .expect(EMBEDDED_FILES_ARE_TESTED);
        bundle
            .add_resource(resource)
            .expect(EMBEDDED_FILES_ARE_TESTED);
    }
    bundle
}

#[cfg(test)]
#[path = "../../../tests/dot-i18n/translator.rs"]
mod tests;
