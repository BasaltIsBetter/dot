use dot_api::Tuning;

use super::*;
use crate::app::tests::IdTranslator;
use crate::settings::tests::test_settings;

const EVERY_SETTING: [Setting; 22] = [
    Setting::Model,
    Setting::Effort,
    Setting::Summaries,
    Setting::Permissions,
    Setting::Folder,
    Setting::Color,
    Setting::StateColors,
    Setting::ToolColors,
    Setting::Language,
    Setting::Size,
    Setting::Column,
    Setting::ReadingLine,
    Setting::Caret,
    Setting::SessionsColumn,
    Setting::Output(OutputPart::Cleaning),
    Setting::Output(OutputPart::Rewriting),
    Setting::Output(OutputPart::Progress),
    Setting::Output(OutputPart::ContentsCut),
    Setting::Output(OutputPart::OutputsCut),
    Setting::Account,
    Setting::ApiKey,
    Setting::SignOut,
];

#[test]
fn lists_every_setting_once_under_its_section() {
    let listed: Vec<Setting> = ENTRIES
        .iter()
        .filter_map(|entry| match entry {
            Entry::Setting(setting) => Some(*setting),
            Entry::Section(_) => None,
        })
        .collect();

    assert_eq!(listed, EVERY_SETTING);
    assert_eq!(ENTRIES[0], Entry::Section(Text::SettingsConversation));
    assert_eq!(ENTRIES[6], Entry::Section(Text::SettingsDisplay));
    assert_eq!(ENTRIES[16], Entry::Section(Text::SettingsOutputs));
    assert_eq!(ENTRIES[22], Entry::Section(Text::SettingsConnection));
}

#[test]
fn chooses_each_option_of_each_setting_and_reads_it_back() {
    for setting in EVERY_SETTING {
        for option in 0..setting.options() {
            let mut settings = test_settings();

            setting.choose(option, &mut settings);

            assert_eq!(setting.chosen(&settings), option, "{setting:?} {option}");
        }
    }
}

#[test]
fn ignores_an_option_past_the_last() {
    for setting in EVERY_SETTING {
        let mut settings = test_settings();

        setting.choose(setting.options(), &mut settings);

        assert_eq!(settings, test_settings(), "{setting:?}");
    }
}

#[test]
fn changes_only_the_setting_chosen() {
    let mut settings = test_settings();

    Setting::Effort.choose(0, &mut settings);

    assert_eq!(
        settings,
        Settings {
            tuning: Tuning {
                effort: Effort::Low,
                ..test_settings().tuning
            },
            ..test_settings()
        }
    );
}

#[test]
fn offers_the_four_models_and_every_effort() {
    assert_eq!(Setting::Model.options(), 4);
    assert_eq!(Setting::Effort.options(), 5);
    assert_eq!(Setting::Model.chosen(&test_settings()), 2);
    assert_eq!(
        Setting::Model.option_name(0, &IdTranslator),
        Model::HAIKU_5_5.name
    );
}

#[test]
fn offers_the_eight_colours_with_the_chosen_one_selected() {
    let mut settings = test_settings();
    assert_eq!(Setting::Color.options(), 8);
    assert_eq!(Setting::Color.chosen(&settings), 6);

    Setting::Color.choose(3, &mut settings);

    assert_eq!(settings.color, Hue::Green);
    assert_eq!(Setting::Color.chosen(&settings), 3);
    Setting::Color.choose(0, &mut settings);
    assert_eq!(settings.color, Hue::Orange);
}

#[test]
fn gives_a_colour_only_to_the_colour_setting() {
    assert_eq!(Setting::Color.hue_of(0), Some(Hue::Orange));
    assert_eq!(Setting::Color.hue_of(6), Some(Hue::Violet));
    assert_eq!(Setting::Color.hue_of(7), Some(Hue::Pink));
    assert_eq!(Setting::Color.hue_of(8), None);
    assert_eq!(Setting::Model.hue_of(0), None);
    assert_eq!(Setting::Color.kind(), Kind::Colour);
    assert_eq!(Setting::Column.kind(), Kind::Choice);
}

#[test]
fn names_each_option_and_nothing_past_the_last() {
    for setting in EVERY_SETTING {
        if setting.kind() == Kind::Action {
            assert_eq!(setting.option_name(0, &IdTranslator), "");
            assert_eq!(setting.options(), 1);
            continue;
        }
        for option in 0..setting.options() {
            assert!(
                !setting.option_name(option, &IdTranslator).is_empty(),
                "{setting:?} {option}"
            );
        }
        assert_eq!(setting.option_name(setting.options(), &IdTranslator), "");
    }
}

#[test]
fn names_options_in_the_user_s_words() {
    let named = |setting: Setting, option| setting.option_name(option, &IdTranslator);

    assert_eq!(named(Setting::Language, 0), "english");
    assert_eq!(named(Setting::Language, 3), "português");
    assert_eq!(
        named(Setting::Column, 1),
        format!("{} 80", Text::ColumnCharacters.id())
    );
    assert_eq!(named(Setting::Permissions, 0), Text::PermissionsBypass.id());
    assert_eq!(named(Setting::Permissions, 1), Text::PermissionsAsk.id());
    assert_eq!(
        named(Setting::Summaries, 0),
        Text::SummariesWhenTheyPay.id()
    );
    assert_eq!(
        named(Setting::Summaries, 1),
        Text::SummariesLeftToClaude.id()
    );
    assert_eq!(named(Setting::Color, 0), Text::HueOrange.id());
    assert_eq!(named(Setting::Color, 6), Text::HueViolet.id());
    assert_eq!(named(Setting::ReadingLine, 0), Text::ReadingLineMiddle.id());
    assert_eq!(named(Setting::ReadingLine, 1), Text::ReadingLineThird.id());
    assert_eq!(named(Setting::Caret, 0), Text::CaretBlinking.id());
    assert_eq!(named(Setting::Caret, 1), Text::CaretSteady.id());
    assert_eq!(
        named(Setting::SessionsColumn, 0),
        Text::SessionsColumnAuto.id()
    );
    assert_eq!(
        named(Setting::SessionsColumn, 1),
        Text::SessionsColumnNever.id()
    );
    assert_eq!(named(Setting::Effort, 4), Effort::Max.text().id());
    let percent = |value: &str| format!("{} {value}", Text::SizePercent.id());
    let sizes: Vec<String> = (0..Setting::Size.options())
        .map(|option| named(Setting::Size, option))
        .collect();
    assert_eq!(sizes, ["80", "90", "100", "115", "130"].map(percent));
}

#[test]
fn gives_each_setting_its_own_label_and_hint() {
    let mut texts: Vec<&str> = EVERY_SETTING
        .iter()
        .flat_map(|setting| [setting.label().id(), setting.hint().id()])
        .collect();
    let count = texts.len();

    texts.sort_unstable();
    texts.dedup();

    assert_eq!(texts.len(), count);
    assert_eq!(Setting::Model.label(), Text::SettingModel);
    assert_eq!(
        Setting::SessionsColumn.hint(),
        Text::SettingSessionsColumnHint
    );
}

#[test]
fn names_each_colour_with_its_own_text() {
    let mut texts: Vec<&str> = Hue::ALL.iter().map(|hue| hue_text(*hue).id()).collect();

    texts.sort_unstable();
    texts.dedup();

    assert_eq!(texts.len(), Hue::ALL.len());
    assert_eq!(hue_text(Hue::Violet), Text::HueViolet);
}
