use super::*;

use crate::language::texts;

fn without_texts() -> FluentTranslator {
    FluentTranslator {
        language: Cell::new(Language::French),
        bundles: [
            OnceCell::from(FluentBundle::new(Vec::new())),
            OnceCell::new(),
        ],
    }
}

fn declared_ids(texts: &str) -> Vec<&str> {
    texts
        .lines()
        .filter(|line| line.starts_with(|first: char| first.is_ascii_lowercase()))
        .filter_map(|line| line.split_once('='))
        .map(|(id, _)| id.trim())
        .collect()
}

#[test]
fn translates_every_text_in_every_language() {
    for language in Language::ALL {
        let translator = FluentTranslator::new(language);
        let mut arguments = FluentArgs::new();
        arguments.set(COUNT, 2);
        arguments.set(VALUE, "layout.rs");

        for text in Text::ALL {
            let message = translator.bundle().get_message(text.id());
            let pattern = message.and_then(|message| message.value());
            assert!(pattern.is_some(), "{} manque en {language:?}", text.id());

            let mut errors = Vec::new();
            let translated =
                translator
                    .bundle()
                    .format_pattern(pattern.unwrap(), Some(&arguments), &mut errors);
            assert!(errors.is_empty(), "{}: {errors:?}", text.id());
            assert!(!translated.trim().is_empty(), "{} est vide", text.id());
        }
    }
}

#[test]
fn every_translation_belongs_to_a_text() {
    let known_ids: Vec<_> = Text::ALL.iter().map(|text| text.id()).collect();

    for language in Language::ALL {
        for id in declared_ids(texts(language)) {
            assert!(
                known_ids.contains(&id),
                "{id} ({language:?}) n'est pas un Text"
            );
        }
    }
}

#[test]
fn reads_the_ids_declared_in_a_file() {
    let texts = "### commentaire\n-app-name = Dot\nmenu-quit = Quitter\n  suite\n";

    assert_eq!(declared_ids(texts), ["menu-quit"]);
}

#[test]
fn writes_the_menu_in_french() {
    let translator = FluentTranslator::new(Language::French);

    assert_eq!(translator.translate(Text::MenuQuit), "Quitter Dot");
    assert_eq!(translator.translate(Text::MenuAbout), "À propos de Dot");
}

#[test]
fn writes_the_menu_in_english() {
    let translator = FluentTranslator::new(Language::English);

    assert_eq!(translator.translate(Text::MenuQuit), "Quit Dot");
    assert_eq!(translator.translate(Text::MenuAbout), "About Dot");
}

#[test]
fn answers_with_the_id_when_a_text_is_missing() {
    let translator = without_texts();

    assert_eq!(translator.translate(Text::MenuQuit), "menu-quit");
}

#[test]
fn counts_lines_in_the_plural_forms_of_french() {
    let translator = FluentTranslator::new(Language::French);

    assert_eq!(translator.count(Text::ToolLines, 0), "ligne");
    assert_eq!(translator.count(Text::ToolLines, 1), "ligne");
    assert_eq!(translator.count(Text::ToolLines, 2), "lignes");
    assert_eq!(translator.count(Text::ToolFiles, 12), "fichiers");
}

#[test]
fn counts_lines_in_the_plural_forms_of_english() {
    let translator = FluentTranslator::new(Language::English);

    assert_eq!(translator.count(Text::ToolLines, 0), "lines");
    assert_eq!(translator.count(Text::ToolLines, 1), "line");
    assert_eq!(translator.count(Text::ToolMatches, 1), "match");
    assert_eq!(translator.count(Text::ToolMatches, 7), "matches");
}

#[test]
fn a_count_on_a_text_without_plural_is_the_text_itself() {
    let translator = FluentTranslator::new(Language::English);

    assert_eq!(translator.count(Text::MenuQuit, 3), "Quit Dot");
    assert_eq!(translator.count(Text::ToolExit, 3), "exit");
}

#[test]
fn counts_with_the_id_when_a_text_is_missing() {
    let translator = without_texts();

    assert_eq!(translator.count(Text::ToolLines, 2), "tool-lines");
}

#[test]
fn writes_a_value_where_each_language_places_it() {
    let french = FluentTranslator::new(Language::French);
    let english = FluentTranslator::new(Language::English);

    assert_eq!(
        french.with(Text::StateReading, "layout.rs"),
        "Claude lit layout.rs"
    );
    assert_eq!(
        english.with(Text::StateReading, "layout.rs"),
        "Claude reads layout.rs"
    );
    assert_eq!(french.with(Text::CostDollars, "2,40"), "2,40\u{a0}$");
    assert_eq!(english.with(Text::CostDollars, "2.40"), "$2.40");
}

#[test]
fn counts_the_agents_claude_waits_for() {
    let french = FluentTranslator::new(Language::French);

    assert_eq!(
        french.count(Text::StateWaitingForAgents, 1),
        "Claude attend 1 agent"
    );
    assert_eq!(
        french.count(Text::StateWaitingForAgents, 3),
        "Claude attend 3 agents"
    );
}

#[test]
fn switches_to_another_language_and_back() {
    let translator = FluentTranslator::new(Language::French);

    translator.use_language(Language::English);
    assert_eq!(translator.translate(Text::MenuQuit), "Quit Dot");

    translator.use_language(Language::French);
    assert_eq!(translator.translate(Text::MenuQuit), "Quitter Dot");
}

#[test]
fn loads_a_language_only_once_it_is_used() {
    let translator = FluentTranslator::new(Language::French);
    assert!(translator.bundles[slot(Language::French)].get().is_some());
    assert!(translator.bundles[slot(Language::English)].get().is_none());

    translator.use_language(Language::English);
    assert!(translator.bundles[slot(Language::English)].get().is_none());

    translator.translate(Text::MenuQuit);
    assert!(translator.bundles[slot(Language::English)].get().is_some());
}

#[test]
fn gives_each_language_a_slot_of_its_own() {
    let slots: Vec<_> = Language::ALL.into_iter().map(slot).collect();

    assert_eq!(slots, [0, 1]);
}
