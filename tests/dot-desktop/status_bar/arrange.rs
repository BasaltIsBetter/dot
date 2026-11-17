use std::collections::BTreeSet;

use dot_api::{Cost, Limits, Model, SessionStatus, Usage};
use proptest::prelude::*;

use super::*;
use crate::status_bar::view::tests::{ShortTranslator, status, with_limits};
use crate::status_bar::view::{Field, modules};

fn module(field: Field, text: &str) -> Module {
    Module {
        field,
        spans: vec![Span::new(text, Ink::Full)],
        gauge: None,
    }
}

fn text_of(strip: &Strip) -> String {
    strip
        .text
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn column_of(strip: &Strip, needle: &str) -> usize {
    let text = text_of(strip);
    let byte = text.find(needle).unwrap();
    text[..byte].chars().count()
}

fn fields_kept(status: &SessionStatus, columns: u16) -> BTreeSet<Field> {
    let all = modules(status, &ShortTranslator);
    let text = text_of(&arrange(all.clone(), columns));
    all.into_iter()
        .filter(|module| {
            let shown: String = module.spans.iter().map(|span| span.text.as_str()).collect();
            text.contains(shown.trim())
        })
        .map(|module| module.field)
        .collect()
}

fn every_field_but(dropped: &[Field]) -> BTreeSet<Field> {
    [
        Field::Model,
        Field::Context,
        Field::Cost,
        Field::Branch,
        Field::Project,
        Field::Lines,
    ]
    .into_iter()
    .filter(|field| !dropped.contains(field))
    .collect()
}

#[test]
fn centres_a_lone_field() {
    let strip = arrange(vec![module(Field::Model, "Opus")], 20);

    assert_eq!(column_of(&strip, "Opus"), 8);
    assert_eq!(strip.dividers, Vec::<u16>::new());
}

#[test]
fn divides_the_groups_by_a_line_between_two_blank_cells() {
    let strip = arrange(
        vec![module(Field::Cost, "$1"), module(Field::Model, "ab")],
        20,
    );

    assert_eq!(column_of(&strip, "ab"), 7);
    assert_eq!(column_of(&strip, "$1"), 11);
    assert_eq!(strip.dividers, [10]);
    assert_eq!(strip.rules, Vec::<u16>::new());
}

#[test]
fn splits_what_belongs_together_by_a_short_rule_not_a_divider() {
    let strip = arrange(
        vec![module(Field::Project, "Dot"), module(Field::Branch, "main")],
        20,
    );

    assert_eq!(column_of(&strip, "Dot"), 5);
    assert_eq!(column_of(&strip, "main"), 10);
    assert_eq!(strip.rules, [9]);
    assert_eq!(strip.dividers, Vec::<u16>::new());
}

#[test]
fn moves_the_gauge_with_its_field() {
    let mut context = module(Field::Context, "       0%");
    context.gauge = Some(Gauge {
        column: 0,
        width: 6,
        fill_per_mille: 0,
        ink: Ink::Receding,
    });

    let strip = arrange(vec![context], 21);

    assert_eq!(strip.gauge.unwrap().column, 6);
}

#[test]
fn shows_every_field_when_the_bar_is_wide_enough() {
    assert_eq!(fields_kept(&status(), 65), every_field_but(&[]));
    assert_eq!(fields_kept(&with_limits(), 81).len(), 8);
}

#[test]
fn drops_the_least_needed_fields_first() {
    let limited = fields_kept(&with_limits(), 80);
    assert!(!limited.contains(&Field::Lines));
    assert!(limited.contains(&Field::WeeklyLimit));

    assert_eq!(fields_kept(&status(), 64), every_field_but(&[Field::Lines]));
    assert_eq!(
        fields_kept(&status(), 55),
        every_field_but(&[Field::Lines, Field::Project])
    );
    assert_eq!(
        fields_kept(&status(), 50),
        every_field_but(&[Field::Lines, Field::Project, Field::Branch])
    );
    assert_eq!(fields_kept(&status(), 10), BTreeSet::from([Field::Model]));
}

#[test]
fn an_empty_bar_holds_nothing() {
    assert_eq!(
        arrange(modules(&status(), &ShortTranslator), 0),
        Strip::default()
    );
}

fn any_status() -> impl Strategy<Value = SessionStatus> {
    (
        proptest::option::of("\\PC{0,40}"),
        proptest::option::of("\\PC{0,40}"),
        any::<[u64; 4]>(),
        any::<[Option<u8>; 2]>(),
    )
        .prop_map(
            |(project, branch, [context, added, removed, cost], [five_hours, seven_days])| {
                SessionStatus {
                    model: Model::OPUS_5_5,
                    project,
                    branch,
                    usage: Usage {
                        context_tokens: context,
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
    fn never_draws_past_the_last_column(status in any_status(), columns in 0_u16..300) {
        let strip = arrange(modules(&status, &ShortTranslator), columns);

        prop_assert!(text_of(&strip).chars().count() <= usize::from(columns));
        prop_assert!(strip.text.spans.iter().all(|span| !span.text.is_empty()));
        prop_assert!(strip.rules.iter().all(|&rule| rule < columns));
        prop_assert!(strip.dividers.iter().all(|&divider| divider < columns));
        if let Some(gauge) = strip.gauge {
            prop_assert!(gauge.column + gauge.width <= columns);
        }
    }
}
