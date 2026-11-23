use dot_api::{SessionStatus, Text, Translator};

use crate::canvas::Icon;
use crate::status_bar::format::{dollars, share, token_count};
use dot_transcript::{Ink, Span, truncated};

const VALUE: Ink = Ink::Full;
const UNIT: Ink = Ink::Secondary;
const LIVE: Ink = Ink::Live;
const EFFORT_MARK: &str = "◇ ";
const LIMIT_PERCENT_MAX: u8 = 100;
const PERCENT: u16 = 100;
const PER_MILLE: u16 = 1_000;
const LONGEST_MODEL: usize = 16;
const LONGEST_PROJECT: usize = 24;
const LONGEST_BRANCH: usize = 24;
const BLANK: &str = " ";

pub(crate) const GAP: u16 = 1;
pub(crate) const ICON_COLUMNS: u16 = 2;
pub(crate) const GAUGE_COLUMNS: u16 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Field {
    Model,
    Context,
    Cost,
    Effort,
    State,
    Sessions,
    Lines,
    Branch,
    Project,
    FiveHourLimit,
    WeeklyLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Zone {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    pub(crate) field: Field,
    pub(crate) spans: Vec<Span>,
    pub(crate) icon: Option<Icon>,
    pub(crate) gauge: Option<u16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Activity {
    pub(crate) text: Option<String>,
    pub(crate) live: bool,
    pub(crate) sessions: Option<String>,
}

impl Field {
    pub(crate) const fn zone(self) -> Zone {
        match self {
            Self::Model | Self::Effort | Self::State => Zone::Left,
            Self::Project | Self::Branch | Self::Lines | Self::Sessions => Zone::Middle,
            Self::FiveHourLimit | Self::WeeklyLimit | Self::Context | Self::Cost => Zone::Right,
        }
    }
}

impl Item {
    pub(crate) fn width(&self) -> u16 {
        let cells: usize = self.spans.iter().map(Span::columns).sum();
        u16::try_from(cells).unwrap_or(u16::MAX)
    }

    pub(crate) fn cut_to(&mut self, columns: u16) {
        if let Some(span) = self.spans.first_mut() {
            span.text = truncated(&span.text, usize::from(columns));
        }
    }

    const fn plain(field: Field, spans: Vec<Span>) -> Self {
        Self {
            field,
            spans,
            icon: None,
            gauge: None,
        }
    }
}

pub(crate) fn items(
    status: &SessionStatus,
    activity: &Activity,
    translator: &impl Translator,
) -> Vec<Item> {
    let mut items = vec![
        Item {
            field: Field::Model,
            spans: vec![
                blank(ICON_COLUMNS + GAP),
                value(truncated(status.model.name, LONGEST_MODEL)),
            ],
            icon: Some(Icon::Model),
            gauge: None,
        },
        Item::plain(
            Field::Effort,
            vec![unit(format!(
                "{EFFORT_MARK}{}",
                translator.translate(status.effort.text())
            ))],
        ),
    ];
    if let Some(text) = activity.text.as_deref().filter(|text| !text.is_empty()) {
        let ink = if activity.live { LIVE } else { UNIT };
        items.push(Item::plain(Field::State, vec![Span::new(text, ink)]));
    }
    if let Some(project) = named(status.project.as_deref()) {
        let project = truncated(project, LONGEST_PROJECT);
        items.push(Item::plain(Field::Project, vec![value(project)]));
    }
    if let Some(branch) = named(status.branch.as_deref()) {
        let branch = truncated(branch, LONGEST_BRANCH);
        items.push(Item::plain(Field::Branch, vec![unit(branch)]));
    }
    let usage = &status.usage;
    let lines = format!("+{}/−{}", usage.lines_added, usage.lines_removed);
    items.push(Item::plain(Field::Lines, vec![value(lines)]));
    if let Some(sessions) = activity.sessions.as_deref() {
        items.push(Item::plain(Field::Sessions, vec![unit(sessions)]));
    }
    items.extend(limits(status, translator));
    items.push(context(status, translator));
    let cost = translator.with(Text::CostDollars, &dollars(usage.cost, translator));
    items.push(Item::plain(Field::Cost, vec![value(cost)]));
    items
}

fn limits<'a>(
    status: &'a SessionStatus,
    translator: &'a impl Translator,
) -> impl Iterator<Item = Item> + 'a {
    [
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
    ]
    .into_iter()
    .filter_map(move |(field, text, spent)| {
        let spent = spent?.min(LIMIT_PERCENT_MAX).to_string();
        let spans = vec![
            unit(format!("{} ", translator.translate(text))),
            value(translator.with(Text::Percent, &spent)),
        ];
        Some(Item::plain(field, spans))
    })
}

fn context(status: &SessionStatus, translator: &impl Translator) -> Item {
    let used = status.usage.context_tokens;
    let window = status.model.context_window;
    let percent = share(used, window, PERCENT).to_string();

    let mut spans = vec![
        blank(GAUGE_COLUMNS + GAP),
        value(translator.with(Text::Percent, &percent)),
        unit(format!(" {}/{}", token_count(used), token_count(window))),
    ];
    if status.usage.base_tokens > 0 {
        let name = translator.translate(Text::StatusBase);
        spans.push(unit(format!(
            " {name} {}",
            token_count(status.usage.base_tokens)
        )));
    }
    Item {
        field: Field::Context,
        spans,
        icon: None,
        gauge: Some(share(used, window, PER_MILLE)),
    }
}

fn named(name: Option<&str>) -> Option<&str> {
    name.filter(|name| !name.is_empty())
}

fn blank(columns: u16) -> Span {
    Span::new(BLANK.repeat(usize::from(columns)), VALUE)
}

fn value(text: impl Into<String>) -> Span {
    Span::new(text, VALUE)
}

fn unit(text: impl Into<String>) -> Span {
    Span::new(text, UNIT)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/view.rs"]
pub(crate) mod tests;
