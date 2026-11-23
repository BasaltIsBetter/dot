use std::borrow::Cow;

use dot_api::{Cost, Limits, Model, Usage};

use super::*;

pub(crate) struct ShortTranslator;

impl Translator for ShortTranslator {
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
        model: Model::OPUS_5_5,
        effort: dot_api::Effort::High,
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
    }
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

    assert_eq!(context.gauge, Some(147));
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
        !fields(&super::items(
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
        },
        &ShortTranslator,
    );
    let empty = items(
        &status(),
        &Activity {
            text: Some(String::new()),
            live: true,
            sessions: None,
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
