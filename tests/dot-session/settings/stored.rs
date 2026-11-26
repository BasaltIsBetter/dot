use dot_api::Tuning;
use proptest::prelude::*;

use super::*;

pub(crate) fn defaults() -> Settings {
    Settings {
        tuning: Tuning {
            model: Model::OPUS_5_5,
            effort: Effort::High,
            summaries: Summaries::WhenTheyPay,
            compaction: dot_api::OutputCompaction::FULL,
        },
        permissions: PermissionMode::Bypass,
        color: Hue::Violet,
        state_colors: Toggle::On,
        tool_colors: Toggle::On,
        language: Language::French,
        size: InterfaceSize::Large,
        column: Column::Wide,
        reading_line: ReadingLine::Middle,
        caret: CaretBlink::Blinking,
        sessions_column: SessionsColumn::Auto,
    }
}

fn everything_changed() -> Settings {
    Settings {
        tuning: Tuning {
            model: Model::FABLE_5_2,
            effort: Effort::Max,
            summaries: Summaries::LeftToClaude,
            compaction: dot_api::OutputCompaction {
                cleaning: Toggle::Off,
                rewriting: Toggle::Off,
                progress: Toggle::Off,
                contents_cut: CutLevel::Never,
                outputs_cut: CutLevel::Early,
            },
        },
        permissions: PermissionMode::Ask,
        color: Hue::Pink,
        state_colors: Toggle::Off,
        tool_colors: Toggle::Off,
        language: Language::English,
        size: InterfaceSize::Largest,
        column: Column::Narrow,
        reading_line: ReadingLine::Third,
        caret: CaretBlink::Steady,
        sessions_column: SessionsColumn::Never,
    }
}

fn written_text(settings: &Settings) -> String {
    String::from_utf8(written(settings, &defaults())).unwrap()
}

#[test]
fn writes_nothing_but_braces_when_every_setting_is_at_its_default() {
    assert_eq!(written_text(&defaults()), "{}\n");
}

#[test]
fn writes_only_what_differs_from_the_defaults() {
    let mut settings = defaults();
    settings.tuning.effort = Effort::Low;
    settings.column = Column::Medium;

    assert_eq!(
        written_text(&settings),
        "{\"column\":\"80\",\"effort\":\"low\"}\n"
    );
}

#[test]
fn names_every_value_it_writes() {
    let text = written_text(&everything_changed());

    for name in [
        "\"model\":\"claude-fable-5-2\"",
        "\"effort\":\"max\"",
        "\"summaries\":\"left-to-claude\"",
        "\"actions\":\"ask\"",
        "\"color\":\"pink\"",
        "\"state-colors\":\"off\"",
        "\"tool-colors\":\"off\"",
        "\"language\":\"en\"",
        "\"column\":\"64\"",
        "\"size\":\"130\"",
        "\"reading-line\":\"third\"",
        "\"caret\":\"steady\"",
        "\"sessions-column\":\"never\"",
        "\"output-cleaning\":\"off\"",
        "\"output-rewriting\":\"off\"",
        "\"output-progress\":\"off\"",
        "\"contents-cut\":\"never\"",
        "\"outputs-cut\":\"early\"",
    ] {
        assert!(text.contains(name), "{name} manque dans {text}");
    }
}

#[test]
fn reads_back_every_setting_it_wrote() {
    let changed = everything_changed();

    assert_eq!(read(&written(&changed, &defaults()), &defaults()), changed);
}

#[test]
fn reads_every_colour_by_its_name() {
    for (hue, name) in HUES {
        let read_back = read(format!("{{\"color\":\"{name}\"}}").as_bytes(), &defaults());

        assert_eq!(read_back.color, hue);
    }
}

#[test]
fn forgets_the_colours_sessions_used_to_have() {
    let read_back = read(
        b"{\"color\":\"session\",\"live-color\":\"green\",\"session-colors\":\"off\"}",
        &defaults(),
    );

    assert_eq!(read_back, defaults());
}

#[test]
fn reads_every_cut_level_by_its_name() {
    for (level, name) in CUT_LEVELS {
        let text = format!("{{\"contents-cut\":\"{name}\",\"outputs-cut\":\"{name}\"}}");
        let read_back = read(text.as_bytes(), &defaults());

        assert_eq!(read_back.tuning.compaction.contents_cut, level);
        assert_eq!(read_back.tuning.compaction.outputs_cut, level);
    }
}

#[test]
fn an_unknown_cut_level_takes_its_default() {
    let read_back = read(
        b"{\"contents-cut\":\"9000\",\"output-progress\":\"no\"}",
        &defaults(),
    );

    assert_eq!(read_back, defaults());
}

#[test]
fn a_missing_setting_takes_its_default() {
    let read_back = read(b"{\"effort\":\"low\"}", &defaults());

    assert_eq!(read_back.tuning.effort, Effort::Low);
    assert_eq!(read_back.tuning.model, Model::OPUS_5_5);
}

#[test]
fn an_unreadable_value_takes_its_default_and_spares_the_others() {
    let read_back = read(
        b"{\"model\":\"claude-opus-4-8\",\"effort\":7,\"column\":\"80\",\"color\":\"teal\"}",
        &defaults(),
    );

    assert_eq!(read_back.tuning.model, Model::OPUS_5_5);
    assert_eq!(read_back.tuning.effort, Effort::High);
    assert_eq!(read_back.color, Hue::Violet);
    assert_eq!(read_back.column, Column::Medium);
}

#[test]
fn ignores_a_key_it_does_not_know() {
    let read_back = read(b"{\"sound\":\"all\",\"caret\":\"steady\"}", &defaults());

    assert_eq!(
        read_back,
        Settings {
            caret: CaretBlink::Steady,
            ..defaults()
        }
    );
}

#[test]
fn anything_but_a_json_object_gives_the_defaults() {
    for written in [
        &b""[..],
        b"not json",
        b"[\"model\"]",
        b"\"low\"",
        b"{\"model\":",
    ] {
        assert_eq!(read(written, &defaults()), defaults());
    }
}

#[test]
fn finds_a_value_by_its_name_and_a_name_by_its_value() {
    assert_eq!(by_name(&TOGGLES, "off"), Some(Toggle::Off));
    assert_eq!(by_name(&TOGGLES, "Off"), None);
    assert_eq!(named(&TOGGLES, Toggle::On), "on");
    assert_eq!(named(&HUES[..1], Hue::Pink), "");
}

fn any_settings() -> impl Strategy<Value = Settings> {
    (
        (
            prop::sample::select(Model::ALL.to_vec()),
            prop::sample::select(Effort::ALL.to_vec()),
            prop::sample::select(Summaries::ALL.to_vec()),
            prop::sample::select(PERMISSIONS.map(|(value, _)| value).to_vec()),
            prop::sample::select(Hue::ALL.to_vec()),
        ),
        (
            prop::sample::select(TOGGLES.map(|(value, _)| value).to_vec()),
            prop::sample::select(TOGGLES.map(|(value, _)| value).to_vec()),
            prop::sample::select(Language::ALL.to_vec()),
            prop::sample::select(COLUMNS.map(|(value, _)| value).to_vec()),
            prop::sample::select(READING_LINES.map(|(value, _)| value).to_vec()),
            prop::sample::select(CARETS.map(|(value, _)| value).to_vec()),
            prop::sample::select(SESSIONS_COLUMNS.map(|(value, _)| value).to_vec()),
        ),
        (
            prop::sample::select(TOGGLES.map(|(value, _)| value).to_vec()),
            prop::sample::select(TOGGLES.map(|(value, _)| value).to_vec()),
            prop::sample::select(TOGGLES.map(|(value, _)| value).to_vec()),
            prop::sample::select(CutLevel::ALL.to_vec()),
            prop::sample::select(CutLevel::ALL.to_vec()),
            prop::sample::select(InterfaceSize::ALL.to_vec()),
        ),
    )
        .prop_map(
            |(
                (model, effort, summaries, permissions, color),
                (states, tools, language, column, reading_line, caret, sessions_column),
                (cleaning, rewriting, progress, contents_cut, outputs_cut, size),
            )| Settings {
                tuning: Tuning {
                    model,
                    effort,
                    summaries,
                    compaction: dot_api::OutputCompaction {
                        cleaning,
                        rewriting,
                        progress,
                        contents_cut,
                        outputs_cut,
                    },
                },
                permissions,
                color,
                state_colors: states,
                tool_colors: tools,
                language,
                size,
                column,
                reading_line,
                caret,
                sessions_column,
            },
        )
}

proptest! {
    #[test]
    fn any_settings_survive_the_round_trip(settings in any_settings(), defaults in any_settings()) {
        prop_assert_eq!(read(&written(&settings, &defaults), &defaults), settings);
    }

    #[test]
    fn never_panics_on_any_bytes(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
        let _ = read(&bytes, &defaults());
    }

    #[test]
    fn never_panics_on_any_value_of_a_known_key(key in "[a-z-]{1,16}", value in ".{0,16}") {
        let object = serde_json::json!({ key: value }).to_string();

        let _ = read(object.as_bytes(), &defaults());
    }
}
