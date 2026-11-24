use dot_api::{Cost, Limits, SessionStatus, Usage};
use proptest::prelude::*;

use super::*;
use crate::canvas::{Icon, Target};
use crate::status_bar::view::tests::{ShortTranslator, items, status, with_limits, writing};
use crate::status_bar::view::{Activity, GAUGE_COLUMNS};
use dot_transcript::columns;

const WIDE: u16 = 120;

fn strip_of(status: &SessionStatus, activity: &Activity, columns: u16, middle: bool) -> Strip {
    arrange(items(status, activity, &ShortTranslator), columns, middle)
}

fn text_of(strip: &Strip) -> String {
    strip
        .text
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn slice(text: &str, from: usize, count: usize) -> String {
    text.chars().skip(from).take(count).collect()
}

#[test]
fn starts_the_left_zone_at_the_first_column_and_ends_the_right_zone_at_the_last() {
    let strip = strip_of(&status(), &Activity::default(), WIDE, true);
    let text = text_of(&strip);

    assert!(text.starts_with("   Opus 5.5 ◇ high"), "{text}");
    assert!(text.ends_with("$2.40 ◇"), "{text}");
    assert_eq!(columns(&text), usize::from(WIDE));
    assert_eq!(strip.icons, [(0, Icon::Model)]);
}

#[test]
fn centres_the_middle_zone_between_two_rules() {
    let strip = strip_of(&status(), &Activity::default(), WIDE, true);

    assert_eq!(strip.rules, [46, 74]);
    assert_eq!(slice(&text_of(&strip), 48, 24), "Dot feat/mac-app +11/−48");
}

#[test]
fn draws_the_gauge_where_the_context_starts() {
    let strip = strip_of(&status(), &Activity::default(), WIDE, true);

    assert_eq!(
        strip.gauge,
        Some(Gauge {
            column: 90,
            fill_per_mille: 147,
            tint: None,
        })
    );
    assert_eq!(slice(&text_of(&strip), 99, 3), "14%");
}

#[test]
fn says_what_claude_does_after_the_effort() {
    let strip = strip_of(&status(), &writing(), WIDE, true);

    assert_eq!(slice(&text_of(&strip), 19, 13), "Claude writes");
}

#[test]
fn cuts_what_claude_does_to_the_room_left_of_the_middle_zone() {
    let long = Activity {
        text: Some("Claude runs cargo test -p dot-desktop --release".to_owned()),
        live: true,
        sessions: None,
        model_heat: 0,
        effort_heat: 0,
        gauge_tint: None,
        gauge_fill: None,
        name: None,
        name_tint: None,
    };

    let text = text_of(&strip_of(&status(), &long, WIDE, true));

    let state = slice(&text, 19, 26);
    assert_eq!(state, "Claude runs cargo test -p…");
    assert_eq!(slice(&text, 46, 2), "  ");
}

#[test]
fn leaves_out_what_claude_does_when_fewer_than_six_columns_are_left() {
    let text = text_of(&strip_of(&status(), &writing(), 50, false));

    assert!(!text.contains("Claude"), "{text}");
    assert!(text.starts_with("   Opus 5.5 ◇ high"), "{text}");
    assert!(text.ends_with("$2.40 ◇"), "{text}");
}

#[test]
fn leaves_out_the_middle_zone_in_a_narrow_window() {
    let strip = strip_of(&status(), &Activity::default(), WIDE, false);

    assert_eq!(strip.rules, Vec::<u16>::new());
    assert!(!text_of(&strip).contains("feat/mac-app"));
}

#[test]
fn lets_the_least_needed_values_go_first_when_room_lacks() {
    let markers = [
        ("7d", Field::WeeklyLimit),
        ("5h", Field::FiveHourLimit),
        ("Dot", Field::Project),
        ("feat/", Field::Branch),
        ("+11", Field::Lines),
        ("high", Field::Effort),
        ("$", Field::Cost),
        ("%", Field::Context),
        ("Opus", Field::Model),
    ];
    let mut gone = Vec::new();

    for columns in (0..=200).rev() {
        let text = text_of(&strip_of(
            &with_limits(),
            &Activity::default(),
            columns,
            true,
        ));
        for (marker, field) in markers {
            if !text.contains(marker) && !gone.contains(&field) {
                gone.push(field);
            }
        }
    }

    assert_eq!(gone, markers.map(|(_, field)| field));
}

#[test]
fn keeps_every_value_when_the_room_allows() {
    let text = text_of(&strip_of(&with_limits(), &writing(), 200, true));

    for marker in [
        "Opus",
        "◇",
        "Claude writes",
        "Dot",
        "feat/",
        "+11",
        "5h 12%",
        "7d 47%",
        "14%",
        "$2.40",
    ] {
        assert!(text.contains(marker), "{marker} in {text}");
    }
}

fn any_status() -> impl Strategy<Value = SessionStatus> {
    (
        proptest::option::of("\\PC{0,40}"),
        proptest::option::of("\\PC{0,40}"),
        any::<[u64; 5]>(),
        any::<[Option<u8>; 2]>(),
    )
        .prop_map(
            |(project, branch, [context, base, added, removed, cost], [five_hours, seven_days])| {
                SessionStatus {
                    project,
                    branch,
                    usage: Usage {
                        context_tokens: context,
                        base_tokens: base,
                        lines_added: added,
                        lines_removed: removed,
                        cost: Cost::from_micro_dollars(cost),
                    },
                    limits: Limits {
                        five_hours,
                        seven_days,
                    },
                }
            },
        )
}

proptest! {
    #[test]
    fn never_draws_past_the_last_column(
        status in any_status(),
        state in proptest::option::of("\\PC{0,80}"),
        columns in 0_u16..300,
        middle in any::<bool>(),
    ) {
        let activity = Activity { text: state, live: true, sessions: None, model_heat: 0, effort_heat: 0, gauge_tint: None, gauge_fill: None, name: None, name_tint: None };

        let strip = strip_of(&status, &activity, columns, middle);

        prop_assert!(text_of(&strip).chars().count() <= usize::from(columns));
        prop_assert!(strip.text.spans.iter().all(|span| !span.text.is_empty()));
        prop_assert!(strip.rules.iter().all(|&rule| rule < columns));
        if let Some(gauge) = strip.gauge {
            prop_assert!(gauge.column + GAUGE_COLUMNS <= columns);
        }
    }
}

#[test]
fn the_left_zone_may_end_right_where_its_room_ends() {
    let shown: Vec<Item> = items(&status(), &Activity::default(), &ShortTranslator)
        .into_iter()
        .filter(|item| item.field.zone() != crate::status_bar::view::Zone::Middle)
        .collect();
    let zones = Zones::of(shown.clone());
    let exact = width_of(&zones.left) + crate::status_bar::view::GAP + width_of(&zones.right);

    assert!(laid_out(&shown, exact).is_some());
    assert!(laid_out(&shown, exact - 1).is_none());
}

#[test]
fn records_where_each_clickable_value_lies() {
    let strip = strip_of(&status(), &Activity::default(), WIDE, true);
    let text: Vec<char> = text_of(&strip).chars().collect();

    let span = |wanted: Target| {
        strip
            .targets
            .iter()
            .find(|(target, _)| *target == wanted)
            .map(|(_, [start, end])| {
                text[usize::from(*start)..usize::from(*end)]
                    .iter()
                    .collect::<String>()
            })
            .unwrap()
    };
    assert_eq!(span(Target::Model).trim_start(), "Opus 5.5");
    assert_eq!(span(Target::Effort), "◇ high");
    assert_eq!(span(Target::Settings), "◇");
    assert_eq!(strip.targets.last().unwrap().1[1], WIDE);
}
