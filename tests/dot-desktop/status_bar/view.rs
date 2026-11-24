use std::borrow::Cow;

use dot_api::{Cost, Limits, Tuning, Usage};

use super::*;
use crate::settings::tests::test_settings;

pub(crate) const OPUS: Tuning = test_settings().tuning;

pub(crate) struct ShortTranslator;

impl Translator for ShortTranslator {
    fn use_language(&self, _language: dot_api::Language) {}

    fn translate(&self, text: Text) -> Cow<'_, str> {
        match text {
            Text::StatusFiveHourLimit => Cow::Borrowed("5h"),
            Text::StatusWeeklyLimit => Cow::Borrowed("7d"),
            Text::StatusBase => Cow::Borrowed("base"),
            Text::EffortHigh => Cow::Borrowed("high"),
            Text::DecimalSeparator => Cow::Borrowed("."),
            other => Cow::Borrowed(other.id()),
        }
    }

    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
    }

    fn with(&self, text: Text, value: &str) -> Cow<'_, str> {
        match text {
            Text::Percent => Cow::Owned(format!("{value}%")),
            Text::CostDollars => Cow::Owned(format!("${value}")),
            other => Cow::Owned(format!("{} {value}", other.id())),
        }
    }
}

pub(crate) fn status() -> SessionStatus {
    SessionStatus {
        project: Some("Dot".to_owned()),
        branch: Some("feat/mac-app".to_owned()),
        usage: Usage {
            context_tokens: 147_000,
            base_tokens: 0,
            lines_added: 11,
            lines_removed: 48,
            cost: Cost::from_micro_dollars(2_400_000),
        },
        limits: Limits::default(),
    }
}

pub(crate) fn with_limits() -> SessionStatus {
    let mut status = status();
    status.limits = Limits {
        five_hours: Some(12),
        seven_days: Some(47),
    };
    status
}

pub(crate) fn writing() -> Activity {
    Activity {
        text: Some("Claude writes".to_owned()),
        live: true,
        sessions: None,
        model_heat: 0,
        effort_heat: 0,
        gauge_tint: None,
        gauge_fill: None,
        name: None,
        name_tint: None,
    }
}

pub(crate) fn items(
    status: &SessionStatus,
    activity: &Activity,
    translator: &impl Translator,
) -> Vec<Item> {
    super::items(status, (activity, &OPUS), translator)
}

pub(crate) fn text_of(item: &Item) -> String {
    item.spans.iter().map(|span| span.text.as_str()).collect()
}

fn item_of(items: &[Item], field: Field) -> Item {
    items
        .iter()
        .find(|item| item.field == field)
        .cloned()
        .unwrap()
}

fn fields(items: &[Item]) -> Vec<Field> {
    items.iter().map(|item| item.field).collect()
}

#[test]
fn lists_each_value_from_left_to_right() {
    let items = items(&status(), &Activity::default(), &ShortTranslator);

    assert_eq!(
        fields(&items),
        [
            Field::Model,
            Field::Effort,
            Field::Project,
            Field::Branch,
            Field::Lines,
            Field::Context,
            Field::Cost,
            Field::Gear,
        ]
    );
}

#[test]
fn places_each_field_in_its_zone() {
    for (field, zone) in [
        (Field::Model, Zone::Left),
        (Field::Effort, Zone::Left),
        (Field::State, Zone::Left),
        (Field::Project, Zone::Middle),
        (Field::Branch, Zone::Middle),
        (Field::Lines, Zone::Middle),
        (Field::FiveHourLimit, Zone::Right),
        (Field::WeeklyLimit, Zone::Right),
        (Field::Context, Zone::Right),
        (Field::Cost, Zone::Right),
    ] {
        assert_eq!(field.zone(), zone, "{field:?}");
    }
}

#[test]
fn names_the_model_after_the_room_of_its_robot() {
    let model = item_of(
        &items(&status(), &Activity::default(), &ShortTranslator),
        Field::Model,
    );

    assert_eq!(text_of(&model), "   Opus 5.5");
    assert_eq!(model.icon, Some(Icon::Model));
    assert_eq!(model.spans[1].ink, Ink::Full);
}

#[test]
fn marks_the_effort_with_a_diamond_in_secondary_ink() {
    let effort = item_of(
        &items(&status(), &Activity::default(), &ShortTranslator),
        Field::Effort,
    );

    assert_eq!(text_of(&effort), "◇ high");
    assert_eq!(effort.spans[0].ink, Ink::Secondary);
}

#[test]
fn writes_the_project_in_full_ink_the_branch_in_secondary_and_the_lines_changed() {
    let items = items(&status(), &Activity::default(), &ShortTranslator);

    let project = item_of(&items, Field::Project);
    let branch = item_of(&items, Field::Branch);
    let lines = item_of(&items, Field::Lines);
    assert_eq!(
        (text_of(&project), project.spans[0].ink),
        ("Dot".to_owned(), Ink::Full)
    );
    assert_eq!(
        (text_of(&branch), branch.spans[0].ink),
        ("feat/mac-app".to_owned(), Ink::Secondary)
    );
    assert_eq!(
        (text_of(&lines), lines.spans[0].ink),
        ("+11/−48".to_owned(), Ink::Full)
    );
}

#[test]
fn leaves_out_a_project_or_a_branch_without_a_name() {
    let mut nameless = status();
    nameless.project = Some(String::new());
    nameless.branch = None;

    let items = items(&nameless, &Activity::default(), &ShortTranslator);

    assert!(!fields(&items).contains(&Field::Project));
    assert!(!fields(&items).contains(&Field::Branch));
}

#[test]
fn shortens_long_names() {
    let mut long = status();
    long.project = Some("p".repeat(40));
    long.branch = Some("b".repeat(40));

    let items = items(&long, &Activity::default(), &ShortTranslator);

    assert_eq!(item_of(&items, Field::Project).width(), 24);
    assert_eq!(item_of(&items, Field::Branch).width(), 24);
}

#[test]
fn draws_a_gauge_before_the_share_of_the_context_and_its_tokens() {
    let context = item_of(
        &items(&status(), &Activity::default(), &ShortTranslator),
        Field::Context,
    );

    assert_eq!(context.gauge, Some((147, None)));
    assert_eq!(text_of(&context), format!("{}14% 147k/1.0M", " ".repeat(9)));
    assert_eq!(context.spans[1].ink, Ink::Full);
    assert_eq!(context.spans[2].ink, Ink::Secondary);
}

#[test]
fn adds_the_base_of_the_conversation_once_measured() {
    let mut measured = status();
    measured.usage.base_tokens = 49_300;

    let context = item_of(
        &items(&measured, &Activity::default(), &ShortTranslator),
        Field::Context,
    );

    assert!(
        text_of(&context).ends_with("147k/1.0M base 49k"),
        "{}",
        text_of(&context)
    );
}

#[test]
fn writes_the_cost_with_the_decimal_separator_and_the_dollar_of_the_language() {
    let cost = item_of(
        &items(&status(), &Activity::default(), &ShortTranslator),
        Field::Cost,
    );

    assert_eq!(text_of(&cost), "$2.40");
    assert_eq!(cost.spans[0].ink, Ink::Full);
}

#[test]
fn shows_the_limits_only_once_known_and_never_past_a_hundred() {
    let mut spent = with_limits();
    spent.limits.seven_days = Some(130);

    let items = items(&spent, &Activity::default(), &ShortTranslator);

    assert_eq!(text_of(&item_of(&items, Field::FiveHourLimit)), "5h 12%");
    assert_eq!(text_of(&item_of(&items, Field::WeeklyLimit)), "7d 100%");
    assert!(
        !fields(&self::items(
            &status(),
            &Activity::default(),
            &ShortTranslator
        ))
        .contains(&Field::WeeklyLimit)
    );
}

#[test]
fn says_what_claude_does_in_orange_while_it_replies() {
    let live = items(&status(), &writing(), &ShortTranslator);
    let quiet = items(
        &status(),
        &Activity {
            text: Some("done".to_owned()),
            live: false,
            sessions: None,
            model_heat: 0,
            effort_heat: 0,
            gauge_tint: None,
            gauge_fill: None,
            name: None,
            name_tint: None,
        },
        &ShortTranslator,
    );
    let empty = items(
        &status(),
        &Activity {
            text: Some(String::new()),
            live: true,
            sessions: None,
            model_heat: 0,
            effort_heat: 0,
            gauge_tint: None,
            gauge_fill: None,
            name: None,
            name_tint: None,
        },
        &ShortTranslator,
    );

    let state = item_of(&live, Field::State);
    assert_eq!(
        (text_of(&state), state.spans[0].ink),
        ("Claude writes".to_owned(), Ink::Live)
    );
    assert_eq!(item_of(&quiet, Field::State).spans[0].ink, Ink::Secondary);
    assert!(!fields(&empty).contains(&Field::State));
    assert_eq!(fields(&live)[2], Field::State);
}

#[test]
fn cuts_an_item_to_its_room_with_an_ellipsis() {
    let mut state = item_of(
        &items(&status(), &writing(), &ShortTranslator),
        Field::State,
    );

    state.cut_to(8);

    assert_eq!(text_of(&state), "Claude …");
    assert_eq!(state.width(), 8);
}

#[test]
fn names_the_model_and_the_effort_chosen_in_the_settings() {
    let tuning = Tuning {
        model: dot_api::Model::HAIKU_5_5,
        effort: dot_api::Effort::Max,
        ..OPUS
    };

    let items = super::items(&status(), (&Activity::default(), &tuning), &ShortTranslator);

    assert_eq!(text_of(&item_of(&items, Field::Model)), "   Haiku 5.5");
    assert_eq!(
        text_of(&item_of(&items, Field::Effort)),
        format!("◇ {}", Text::EffortMax.id())
    );
}

#[test]
fn warms_the_model_and_the_effort_just_tuned_each_on_its_own() {
    let warm = Activity {
        model_heat: 200,
        effort_heat: 90,
        ..Activity::default()
    };

    let warmed = items(&status(), &warm, &ShortTranslator);
    let cold = items(&status(), &Activity::default(), &ShortTranslator);

    assert_eq!(item_of(&warmed, Field::Model).spans[1].heat, 200);
    assert_eq!(item_of(&warmed, Field::Effort).spans[0].heat, 90);
    assert_eq!(item_of(&warmed, Field::Cost).spans[0].heat, 0);
    assert_eq!(item_of(&cold, Field::Model).spans[1].heat, 0);
}

#[test]
fn aims_the_model_the_effort_and_the_gear_at_the_settings() {
    let items = items(&status(), &Activity::default(), &ShortTranslator);

    let aimed: Vec<(Field, Target)> = items
        .iter()
        .filter_map(|item| item.target.map(|target| (item.field, target)))
        .collect();
    assert_eq!(
        aimed,
        [
            (Field::Model, Target::Model),
            (Field::Effort, Target::Effort),
            (Field::Gear, Target::Settings),
        ]
    );
    let gear = item_of(&items, Field::Gear);
    assert_eq!(text_of(&gear), "◇");
    assert_eq!(gear.spans[0].ink, Ink::Faint);
    assert_eq!(Field::Gear.zone(), Zone::Right);
}

fn searching(query: &str, matching: usize, saved_heat: u8) -> Searching {
    Searching {
        query: query.to_owned(),
        matching,
        saved_heat,
    }
}

#[test]
fn titles_the_bar_of_the_settings_and_tells_each_change_is_saved() {
    let items = searching_items(&searching("", 10, 0), &ShortTranslator);

    assert_eq!(
        fields(&items),
        [
            Field::SettingsTitle,
            Field::SettingsSaved,
            Field::SettingsKeys
        ]
    );
    assert_eq!(
        text_of(&item_of(&items, Field::SettingsTitle)),
        format!("◇ {}", Text::SettingsTitle.id())
    );
    let saved = item_of(&items, Field::SettingsSaved);
    assert_eq!(text_of(&saved), Text::SettingsSaved.id());
    assert_eq!(saved.spans[0].heat, 0);
}

#[test]
fn shows_the_search_in_live_ink_with_its_count() {
    let items = searching_items(&searching("ef", 1, 0), &ShortTranslator);

    let query = item_of(&items, Field::SettingsQuery);
    assert_eq!(text_of(&query), "⌕ ef_ · 1");
    assert_eq!(query.spans[0].ink, Ink::Live);
    assert_eq!(Field::SettingsQuery.zone(), Zone::Left);
}

#[test]
fn warms_saved_just_after_a_change() {
    let items = searching_items(&searching("", 10, 180), &ShortTranslator);

    assert_eq!(item_of(&items, Field::SettingsSaved).spans[0].heat, 180);
    assert_eq!(Field::SettingsSaved.zone(), Zone::Middle);
}

#[test]
fn recalls_the_keys_and_closes_on_a_click() {
    let items = searching_items(&searching("", 10, 0), &ShortTranslator);

    let keys = item_of(&items, Field::SettingsKeys);
    assert_eq!(
        text_of(&keys),
        format!(
            "↑↓ {}  ←→ {}  ⌫ {}  {} {}  ◇",
            Text::SettingsChoose.id(),
            Text::SettingsChange.id(),
            Text::SettingsReset.id(),
            Text::HelpStopKey.id(),
            Text::SettingsClose.id()
        )
    );
    assert_eq!(keys.target, Some(Target::Settings));
    assert_eq!(Field::SettingsKeys.zone(), Zone::Right);
    assert_eq!(Field::SettingsTitle.zone(), Zone::Left);
}

#[test]
fn colours_the_share_of_the_context_and_its_gauge_in_the_tint_given() {
    let tinted = Activity {
        gauge_tint: Some([1, 2, 3]),
        ..Activity::default()
    };

    let items = items(&status(), &tinted, &ShortTranslator);
    let plain = items_of_default();

    let context = item_of(&items, Field::Context);
    assert_eq!(context.spans[1].ink, Ink::Rgb([1, 2, 3]));
    assert_eq!(context.gauge.map(|(_, tint)| tint), Some(Some([1, 2, 3])));
    let plain_context = item_of(&plain, Field::Context);
    assert_eq!(plain_context.spans[1].ink, Ink::Full);
    assert_eq!(plain_context.gauge.map(|(_, tint)| tint), Some(None));
}

fn items_of_default() -> Vec<Item> {
    items(&status(), &Activity::default(), &ShortTranslator)
}

#[test]
fn shows_the_session_s_own_name_in_place_of_the_folder_warm_once_renamed() {
    let renamed = Activity {
        name: Some("api".to_owned()),
        name_tint: Some([1, 2, 3]),
        ..Activity::default()
    };

    let shown = items(&status(), &renamed, &ShortTranslator);

    let project = item_of(&shown, Field::Project);
    assert_eq!(text_of(&project), "api");
    assert_eq!(project.spans[0].ink, Ink::Rgb([1, 2, 3]));
    let folder = item_of(&items_of_default(), Field::Project);
    assert_eq!(text_of(&folder), "Dot");
    assert_eq!(folder.spans[0].ink, Ink::Full);
    let unnamed = Activity {
        name_tint: Some([1, 2, 3]),
        ..Activity::default()
    };
    let folder = item_of(
        &items(&status(), &unnamed, &ShortTranslator),
        Field::Project,
    );
    assert_eq!(folder.spans[0].ink, Ink::Full);
}
