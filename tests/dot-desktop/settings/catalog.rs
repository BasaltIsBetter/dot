use dot_api::Tuning;

use super::*;
use crate::app::tests::IdTranslator;
use crate::settings::tests::test_settings;

const EVERY_SETTING: [Setting; 17] = [
    Setting::Model,
    Setting::Effort,
    Setting::Summaries,
    Setting::Permissions,
    Setting::Folder,
    Setting::LiveColor,
    Setting::StateColors,
    Setting::ToolColors,
    Setting::SessionColors,
    Setting::Language,
    Setting::Column,
    Setting::ReadingLine,
    Setting::Caret,
    Setting::SessionsColumn,
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
    assert_eq!(ENTRIES[16], Entry::Section(Text::SettingsConnection));
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
fn offers_the_session_colour_first_then_the_eight_colours() {
    let mut settings = test_settings();
    assert_eq!(Setting::LiveColor.options(), 9);
    assert_eq!(Setting::LiveColor.chosen(&settings), 0);

    Setting::LiveColor.choose(4, &mut settings);

    assert_eq!(settings.live_color, LiveColor::Fixed(Hue::Green));
    Setting::LiveColor.choose(0, &mut settings);
    assert_eq!(settings.live_color, LiveColor::OfTheSession);
}

#[test]
fn gives_a_colour_only_to_the_live_colour() {
    assert_eq!(Setting::LiveColor.hue_of(0, Hue::Cyan), Some(Hue::Cyan));
    assert_eq!(Setting::LiveColor.hue_of(1, Hue::Cyan), Some(Hue::Orange));
    assert_eq!(Setting::LiveColor.hue_of(8, Hue::Cyan), Some(Hue::Pink));
    assert_eq!(Setting::LiveColor.hue_of(9, Hue::Cyan), None);
    assert_eq!(Setting::Model.hue_of(0, Hue::Cyan), None);
    assert_eq!(Setting::LiveColor.kind(), Kind::Colour);
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

    assert_eq!(named(Setting::Language, 0), "français");
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
    assert_eq!(
        named(Setting::LiveColor, 0),
        Text::LiveColorOfTheSession.id()
    );
    assert_eq!(named(Setting::LiveColor, 2), Text::HueRed.id());
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
