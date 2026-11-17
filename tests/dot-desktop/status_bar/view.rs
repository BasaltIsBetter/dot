use std::borrow::Cow;

use dot_api::{Cost, Limits, Model, Usage};

use super::*;

pub(crate) struct ShortTranslator;

impl Translator for ShortTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        match text {
            Text::StatusFiveHourLimit => Cow::Borrowed("5h"),
            Text::StatusWeeklyLimit => Cow::Borrowed("7d"),
            other => Cow::Borrowed(other.id()),
        }
    }
}

pub(crate) fn status() -> SessionStatus {
    SessionStatus {
        model: Model::OPUS_5_5,
        project: Some("Dot".to_owned()),
        branch: Some("feat/mac-app".to_owned()),
        usage: Usage {
            context_tokens: 147_000,
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

fn text_of(module: &Module) -> String {
    module.spans.iter().map(|span| span.text.as_str()).collect()
}

fn module_of(modules: &[Module], field: Field) -> Module {
    modules
        .iter()
        .find(|module| module.field == field)
        .cloned()
        .unwrap()
}

fn fields(modules: &[Module]) -> Vec<Field> {
    modules.iter().map(|module| module.field).collect()
}

fn context_of(status: &SessionStatus) -> Module {
    module_of(&modules(status, &ShortTranslator), Field::Context)
}

#[test]
fn shows_every_field_of_the_claude_code_status_line() {
    let shown: Vec<(Field, String)> = modules(&with_limits(), &ShortTranslator)
        .iter()
        .map(|module| (module.field, text_of(module)))
        .collect();

    assert_eq!(
        shown,
        [
            (Field::Model, "Opus 5.5".to_owned()),
            (Field::Context, "       14% 147k/1.0M".to_owned()),
            (Field::Lines, "+11 −48".to_owned()),
            (Field::Cost, "$2.40".to_owned()),
            (Field::Project, "Dot".to_owned()),
            (Field::Branch, "feat/mac-app".to_owned()),
            (Field::FiveHourLimit, "5h 12%".to_owned()),
            (Field::WeeklyLimit, "7d 47%".to_owned()),
        ]
    );
}

#[test]
fn leaves_out_a_limit_until_it_has_a_value() {
    let none = modules(&status(), &ShortTranslator);
    let mut weekly_only = status();
    weekly_only.limits.seven_days = Some(3);

    assert!(!fields(&none).contains(&Field::FiveHourLimit));
    assert!(!fields(&none).contains(&Field::WeeklyLimit));
    let weekly = modules(&weekly_only, &ShortTranslator);
    assert!(!fields(&weekly).contains(&Field::FiveHourLimit));
    assert_eq!(text_of(&module_of(&weekly, Field::WeeklyLimit)), "7d 3%");
}

#[test]
fn never_shows_a_limit_above_a_hundred_percent() {
    let mut status = status();
    status.limits.five_hours = Some(250);

    let modules = modules(&status, &ShortTranslator);

    assert_eq!(
        text_of(&module_of(&modules, Field::FiveHourLimit)),
        "5h 100%"
    );
}

#[test]
fn leaves_the_branch_out_of_a_folder_outside_git() {
    let mut status = status();
    status.branch = None;

    assert!(!fields(&modules(&status, &ShortTranslator)).contains(&Field::Branch));
}

#[test]
fn takes_an_empty_name_for_no_name() {
    let mut status = status();
    status.project = Some(String::new());
    status.branch = Some(String::new());

    let modules = modules(&status, &ShortTranslator);

    assert_eq!(text_of(&module_of(&modules, Field::Project)), "—");
    assert!(!fields(&modules).contains(&Field::Branch));
}

#[test]
fn dashes_the_project_of_a_session_without_folder() {
    let mut status = status();
    status.project = None;

    let modules = modules(&status, &ShortTranslator);

    assert_eq!(text_of(&module_of(&modules, Field::Project)), "—");
}

#[test]
fn writes_values_in_full_ink_and_their_units_receding() {
    let modules = modules(&with_limits(), &ShortTranslator);
    let inks = |field| -> Vec<(String, Ink)> {
        module_of(&modules, field)
            .spans
            .into_iter()
            .map(|span| (span.text, span.ink))
            .collect()
    };

    assert_eq!(
        inks(Field::Cost),
        [
            ("$".to_owned(), Ink::Receding),
            ("2.40".to_owned(), Ink::Full)
        ]
    );
    assert_eq!(
        inks(Field::Context)[1..],
        [
            ("14".to_owned(), Ink::Full),
            ("% ".to_owned(), Ink::Receding),
            ("147k".to_owned(), Ink::Full),
            ("/1.0M".to_owned(), Ink::Receding),
        ]
    );
    assert_eq!(
        inks(Field::WeeklyLimit),
        [
            ("7d ".to_owned(), Ink::Receding),
            ("47".to_owned(), Ink::Full),
            ("%".to_owned(), Ink::Receding),
        ]
    );
}

#[test]
fn draws_the_context_gauge_before_the_percentage() {
    let context = context_of(&status());

    assert_eq!(
        context.gauge,
        Some(Gauge {
            column: 0,
            width: 6,
            fill_per_mille: 147,
            ink: Ink::Receding,
        })
    );
    assert_eq!(context.spans[0].text, " ".repeat(7));
}

#[test]
fn brightens_the_gauge_as_the_context_fills() {
    let ink_at = |tokens| {
        let mut status = status();
        status.usage.context_tokens = tokens;
        context_of(&status).gauge.unwrap().ink
    };

    assert_eq!(ink_at(499_999), Ink::Receding);
    assert_eq!(ink_at(500_000), Ink::Secondary);
    assert_eq!(ink_at(799_999), Ink::Secondary);
    assert_eq!(ink_at(800_000), Ink::Full);
}

#[test]
fn starts_a_new_session_at_zero() {
    let mut status = status();
    status.usage = Usage::default();

    let modules = modules(&status, &ShortTranslator);

    assert_eq!(
        text_of(&module_of(&modules, Field::Context)),
        "       0% 0k/1.0M"
    );
    assert_eq!(text_of(&module_of(&modules, Field::Lines)), "+0 −0");
    assert_eq!(text_of(&module_of(&modules, Field::Cost)), "$0.00");
}

#[test]
fn shortens_long_names() {
    let mut status = status();
    status.project = Some("p".repeat(30));
    status.branch = Some("b".repeat(30));

    let modules = modules(&status, &ShortTranslator);

    assert_eq!(module_of(&modules, Field::Project).width(), 20);
    assert_eq!(module_of(&modules, Field::Branch).width(), 24);
    assert!(text_of(&module_of(&modules, Field::Branch)).ends_with('…'));
}

#[test]
fn measures_a_module_in_cells_not_bytes() {
    let modules = modules(&status(), &ShortTranslator);

    assert_eq!(module_of(&modules, Field::Lines).width(), 7);
}

#[test]
fn keeps_what_belongs_together_in_one_group() {
    assert_eq!(Field::Project.group(), Field::Branch.group());
    assert_eq!(Field::FiveHourLimit.group(), Field::WeeklyLimit.group());
    let alone = [Field::Model, Field::Context, Field::Lines, Field::Cost];
    for (index, field) in alone.iter().enumerate() {
        for other in &alone[index + 1..] {
            assert_ne!(field.group(), other.group());
        }
        assert_ne!(field.group(), Field::Project.group());
        assert_ne!(field.group(), Field::WeeklyLimit.group());
    }
    assert_ne!(Field::Project.group(), Field::WeeklyLimit.group());
}
