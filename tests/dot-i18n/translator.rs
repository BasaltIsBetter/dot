use super::*;

use crate::language::texts;

fn without_texts() -> FluentTranslator {
    FluentTranslator {
        language: Cell::new(Language::English),
        bundles: [
            OnceCell::from(FluentBundle::new(Vec::new())),
            OnceCell::new(),
            OnceCell::new(),
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
            assert!(
                pattern.is_some(),
                "{} is missing in {language:?}",
                text.id()
            );

            let mut errors = Vec::new();
            let translated =
                translator
                    .bundle()
                    .format_pattern(pattern.unwrap(), Some(&arguments), &mut errors);
            assert!(errors.is_empty(), "{}: {errors:?}", text.id());
            assert!(!translated.trim().is_empty(), "{} is empty", text.id());
        }
    }
}

#[test]
fn every_translation_belongs_to_a_text() {
    let known_ids: Vec<_> = Text::ALL.iter().map(|text| text.id()).collect();

    for language in Language::ALL {
        for id in declared_ids(texts(language)) {
            assert!(known_ids.contains(&id), "{id} ({language:?}) is not a Text");
        }
    }
}

#[test]
fn reads_the_ids_declared_in_a_file() {
    let texts = "### comment\n-app-name = Dot\nmenu-quit = Quit\n  more\n";

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
fn writes_the_menu_in_spanish() {
    let translator = FluentTranslator::new(Language::Spanish);

    assert_eq!(translator.translate(Text::MenuQuit), "Salir de Dot");
    assert_eq!(translator.translate(Text::MenuAbout), "Acerca de Dot");
}

#[test]
fn writes_the_menu_in_portuguese() {
    let translator = FluentTranslator::new(Language::Portuguese);

    assert_eq!(translator.translate(Text::MenuQuit), "Encerrar Dot");
    assert_eq!(translator.translate(Text::MenuAbout), "Sobre o Dot");
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
fn counts_zero_in_the_plural_in_spanish() {
    let translator = FluentTranslator::new(Language::Spanish);

    assert_eq!(translator.count(Text::ToolLines, 0), "líneas");
    assert_eq!(translator.count(Text::ToolLines, 1), "línea");
    assert_eq!(translator.count(Text::ToolLines, 2), "líneas");
    assert_eq!(translator.count(Text::AgentRead, 1), "1 archivo leído");
}

#[test]
fn counts_zero_in_the_singular_in_brazilian_portuguese() {
    let translator = FluentTranslator::new(Language::Portuguese);

    assert_eq!(translator.count(Text::ToolLines, 0), "linha");
    assert_eq!(translator.count(Text::ToolLines, 1), "linha");
    assert_eq!(translator.count(Text::ToolLines, 2), "linhas");
    assert_eq!(translator.count(Text::AgentRead, 3), "3 arquivos lidos");
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
    assert_eq!(
        FluentTranslator::new(Language::Spanish).with(Text::CostDollars, "2,40"),
        "2,40\u{a0}US$"
    );
    assert_eq!(
        FluentTranslator::new(Language::Portuguese).with(Text::CostDollars, "2,40"),
        "US$\u{a0}2,40"
    );
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
fn counts_the_agents_out_of_sight() {
    let french = FluentTranslator::new(Language::French);
    let english = FluentTranslator::new(Language::English);

    assert_eq!(french.count(Text::AgentsOutOfSight, 1), "1 agent");
    assert_eq!(french.count(Text::AgentsOutOfSight, 2), "2 agents");
    assert_eq!(english.count(Text::AgentsOutOfSight, 1), "1 agent");
    assert_eq!(english.count(Text::AgentsOutOfSight, 3), "3 agents");
    assert_eq!(french.translate(Text::InstrumentAgents), "Agents");
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

    assert_eq!(slots, [0, 1, 2, 3]);
}
