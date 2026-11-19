use dot_api::{SessionStatus, Text, Translator};

use crate::canvas::{Gauge, Icon, Ink, Span};
use crate::status_bar::format::{dollars, share, token_count, truncated};

const LIMIT_PERCENT_MAX: u8 = 100;
const VALUE: Ink = Ink::Full;
const LABEL: Ink = Ink::Receding;
const GAUGE_CELLS: u16 = 6;
const GAUGE_GAP: u16 = 1;
const BLANK: &str = " ";
const ICON_CELLS: u16 = 3;
const PERCENT: u16 = 100;
const PER_MILLE: u16 = 1_000;
const BUSY_PERCENT: u16 = 50;
const NEARLY_FULL_PERCENT: u16 = 80;
const LONGEST_MODEL: usize = 16;
const LONGEST_BRANCH: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Field {
    Model,
    Effort,
    Context,
    Cost,
    Branch,
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
    Branch,
    Limits,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Module {
    pub(crate) field: Field,
    pub(crate) spans: Vec<Span>,
    pub(crate) icon: Option<Icon>,
    pub(crate) gauge: Option<Gauge>,
}

impl Field {
    pub(crate) const fn group(self) -> Group {
        match self {
            Self::Model | Self::Effort => Group::Model,
            Self::Context => Group::Context,
            Self::Lines => Group::Lines,
            Self::Cost => Group::Cost,
            Self::Branch => Group::Branch,
            Self::FiveHourLimit | Self::WeeklyLimit => Group::Limits,
        }
    }
}

impl Module {
    pub(crate) fn width(&self) -> u16 {
        let cells: usize = self.spans.iter().map(Span::columns).sum();
        u16::try_from(cells).unwrap_or(u16::MAX)
    }

    fn marked(field: Field, icon: Icon, mut spans: Vec<Span>) -> Self {
        spans.insert(0, icon_room());
        Self {
            field,
            spans,
            icon: Some(icon),
            gauge: None,
        }
    }
}

pub(crate) fn modules(status: &SessionStatus, translator: &impl Translator) -> Vec<Module> {
    let mut modules = vec![
        Module::marked(
            Field::Model,
            Icon::Model,
            vec![value(truncated(status.model.name, LONGEST_MODEL))],
        ),
        Module::marked(
            Field::Effort,
            Icon::Effort(status.effort),
            vec![
                label(format!("{} ", translator.translate(Text::StatusEffort))),
                value(translator.translate(status.effort.text())),
            ],
        ),
        context(status),
        lines(status),
        Module::marked(
            Field::Cost,
            Icon::Dollar,
            vec![value(dollars(status.usage.cost))],
        ),
    ];
    if let Some(branch) = named(status.branch.as_deref()) {
        modules.push(Module::marked(
            Field::Branch,
            Icon::Branch,
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
            modules.push(Module::marked(
                field,
                Icon::Clock,
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
            icon_room(),
            Span::new(BLANK.repeat(usize::from(GAUGE_CELLS + GAUGE_GAP)), VALUE),
            value(percent.to_string()),
            label("% "),
            value(token_count(used)),
            label(format!("/{}", token_count(window))),
        ],
        icon: Some(Icon::Battery),
        gauge: Some(Gauge {
            column: ICON_CELLS,
            width: GAUGE_CELLS,
            fill_per_mille: share(used, window, PER_MILLE),
            ink: pressure(percent),
        }),
    }
}

fn lines(status: &SessionStatus) -> Module {
    Module::marked(
        Field::Lines,
        Icon::Lines,
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

fn icon_room() -> Span {
    Span::new(BLANK.repeat(usize::from(ICON_CELLS)), VALUE)
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
