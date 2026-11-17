use dot_api::{SessionStatus, Text, Translator};

use crate::canvas::{Gauge, Ink, Span};
use crate::status_bar::format::{dollars, share, token_count, truncated};

const UNKNOWN: &str = "—";
const LIMIT_PERCENT_MAX: u8 = 100;
const VALUE: Ink = Ink::Full;
const LABEL: Ink = Ink::Receding;
const GAUGE_CELLS: u16 = 6;
const GAUGE_GAP: u16 = 1;
const BLANK: &str = " ";
const PERCENT: u16 = 100;
const PER_MILLE: u16 = 1_000;
const BUSY_PERCENT: u16 = 50;
const NEARLY_FULL_PERCENT: u16 = 80;
const LONGEST_MODEL: usize = 16;
const LONGEST_PROJECT: usize = 20;
const LONGEST_BRANCH: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Field {
    Model,
    Context,
    Cost,
    Branch,
    Project,
    FiveHourLimit,
    WeeklyLimit,
    Lines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Group {
    Model,
    Context,
    Lines,
    Cost,
    Place,
    Limits,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Module {
    pub(crate) field: Field,
    pub(crate) spans: Vec<Span>,
    pub(crate) gauge: Option<Gauge>,
}

impl Field {
    pub(crate) const fn group(self) -> Group {
        match self {
            Self::Model => Group::Model,
            Self::Context => Group::Context,
            Self::Lines => Group::Lines,
            Self::Cost => Group::Cost,
            Self::Project | Self::Branch => Group::Place,
            Self::FiveHourLimit | Self::WeeklyLimit => Group::Limits,
        }
    }
}

impl Module {
    pub(crate) fn width(&self) -> u16 {
        let cells: usize = self
            .spans
            .iter()
            .map(|span| span.text.chars().count())
            .sum();
        u16::try_from(cells).unwrap_or(u16::MAX)
    }

    const fn text(field: Field, spans: Vec<Span>) -> Self {
        Self {
            field,
            spans,
            gauge: None,
        }
    }
}

pub(crate) fn modules(status: &SessionStatus, translator: &impl Translator) -> Vec<Module> {
    let project = named(status.project.as_deref()).unwrap_or(UNKNOWN);
    let mut modules = vec![
        Module::text(
            Field::Model,
            vec![value(truncated(status.model.name, LONGEST_MODEL))],
        ),
        context(status),
        lines(status),
        Module::text(
            Field::Cost,
            vec![label("$"), value(dollars(status.usage.cost))],
        ),
        Module::text(
            Field::Project,
            vec![value(truncated(project, LONGEST_PROJECT))],
        ),
    ];
    if let Some(branch) = named(status.branch.as_deref()) {
        modules.push(Module::text(
            Field::Branch,
            vec![value(truncated(branch, LONGEST_BRANCH))],
        ));
    }
    let limits = [
        (
            Field::FiveHourLimit,
            Text::StatusFiveHourLimit,
            status.limits.five_hours,
        ),
        (
            Field::WeeklyLimit,
            Text::StatusWeeklyLimit,
            status.limits.seven_days,
        ),
    ];
    for (field, text, spent) in limits {
        if let Some(spent) = spent {
            let name = translator.translate(text);
            modules.push(Module::text(
                field,
                vec![
                    label(format!("{name} ")),
                    value(spent.min(LIMIT_PERCENT_MAX).to_string()),
                    label("%"),
                ],
            ));
        }
    }
    modules
}

fn context(status: &SessionStatus) -> Module {
    let used = status.usage.context_tokens;
    let window = status.model.context_window;
    let percent = share(used, window, PERCENT);

    Module {
        field: Field::Context,
        spans: vec![
            Span::new(BLANK.repeat(usize::from(GAUGE_CELLS + GAUGE_GAP)), VALUE),
            value(percent.to_string()),
            label("% "),
            value(token_count(used)),
            label(format!("/{}", token_count(window))),
        ],
        gauge: Some(Gauge {
            column: 0,
            width: GAUGE_CELLS,
            fill_per_mille: share(used, window, PER_MILLE),
            ink: pressure(percent),
        }),
    }
}

fn lines(status: &SessionStatus) -> Module {
    Module::text(
        Field::Lines,
        vec![
            label("+"),
            value(status.usage.lines_added.to_string()),
            label(" −"),
            value(status.usage.lines_removed.to_string()),
        ],
    )
}

const fn pressure(percent: u16) -> Ink {
    if percent >= NEARLY_FULL_PERCENT {
        Ink::Full
    } else if percent >= BUSY_PERCENT {
        Ink::Secondary
    } else {
        Ink::Receding
    }
}

fn named(name: Option<&str>) -> Option<&str> {
    name.filter(|name| !name.is_empty())
}

fn value(text: impl Into<String>) -> Span {
    Span::new(text, VALUE)
}

fn label(text: impl Into<String>) -> Span {
    Span::new(text, LABEL)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/view.rs"]
pub(crate) mod tests;
