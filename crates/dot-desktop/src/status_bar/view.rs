use dot_api::{SessionStatus, Text, Translator, Tuning};

use crate::canvas::{Icon, Target};
use crate::status_bar::format::{dollars, share, token_count};
use dot_transcript::{Ink, Span, truncated};

const VALUE: Ink = Ink::Full;
const UNIT: Ink = Ink::Secondary;
const LIVE: Ink = Ink::Live;
const GEAR: Ink = Ink::Faint;
const EFFORT_MARK: &str = "◇ ";
const SETTINGS_MARK: &str = "◇";
const QUERY_MARK: &str = "⌕ ";
const QUERY_CARET: &str = "_";
const QUERY_COUNT: &str = " · ";
const KEYS: [(Option<&str>, Text); 4] = [
    (Some("↑↓"), Text::SettingsChoose),
    (Some("←→"), Text::SettingsChange),
    (Some("⌫"), Text::SettingsReset),
    (None, Text::SettingsClose),
];
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
    SettingsTitle,
    Model,
    Gear,
    SettingsQuery,
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
    SettingsSaved,
    SettingsKeys,
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
    pub(crate) gauge: Option<(u16, Option<[u8; 3]>)>,
    pub(crate) target: Option<Target>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Activity {
    pub(crate) text: Option<String>,
    pub(crate) live: bool,
    pub(crate) sessions: Option<String>,
    pub(crate) model_heat: u8,
    pub(crate) effort_heat: u8,
    pub(crate) gauge_tint: Option<[u8; 3]>,
    pub(crate) gauge_fill: Option<u16>,
    pub(crate) name: Option<String>,
    pub(crate) name_tint: Option<[u8; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Searching {
    pub(crate) query: String,
    pub(crate) matching: usize,
    pub(crate) saved_heat: u8,
}

impl Field {
    pub(crate) const fn zone(self) -> Zone {
        match self {
            Self::Model
            | Self::Effort
            | Self::State
            | Self::SettingsTitle
            | Self::SettingsQuery => Zone::Left,
            Self::Project | Self::Branch | Self::Lines | Self::Sessions | Self::SettingsSaved => {
                Zone::Middle
            }
            Self::FiveHourLimit
            | Self::WeeklyLimit
            | Self::Context
            | Self::Cost
            | Self::Gear
            | Self::SettingsKeys => Zone::Right,
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
            target: None,
        }
    }

    const fn aiming(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }
}

pub(crate) fn items(
    status: &SessionStatus,
    (activity, tuning): (&Activity, &Tuning),
    translator: &impl Translator,
) -> Vec<Item> {
    let mut items = vec![
        Item {
            field: Field::Model,
            spans: vec![
                blank(ICON_COLUMNS + GAP),
                value(truncated(tuning.model.name, LONGEST_MODEL)).heated(activity.model_heat),
            ],
            icon: Some(Icon::Model),
            gauge: None,
            target: Some(Target::Model),
        },
        Item::plain(
            Field::Effort,
            vec![
                unit(format!(
                    "{EFFORT_MARK}{}",
                    translator.translate(tuning.effort.text())
                ))
                .heated(activity.effort_heat),
            ],
        )
        .aiming(Target::Effort),
    ];
    if let Some(text) = activity.text.as_deref().filter(|text| !text.is_empty()) {
        let ink = if activity.live { LIVE } else { UNIT };
        items.push(Item::plain(Field::State, vec![Span::new(text, ink)]));
    }
    let name = activity.name.as_deref().or(status.project.as_deref());
    if let Some(project) = named(name) {
        let project = truncated(project, LONGEST_PROJECT);
        let span = match activity.name_tint.filter(|_| activity.name.is_some()) {
            Some(tint) => Span::new(project, Ink::Rgb(tint)),
            None => value(project),
        };
        items.push(Item::plain(Field::Project, vec![span]));
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
    items.push(context(status, (tuning, activity), translator));
    let cost = translator.with(Text::CostDollars, &dollars(usage.cost, translator));
    items.push(Item::plain(Field::Cost, vec![value(cost)]));
    items.push(Item::plain(Field::Gear, vec![quiet(SETTINGS_MARK)]).aiming(Target::Settings));
    items
}

pub(crate) fn searching_items(searching: &Searching, translator: &impl Translator) -> Vec<Item> {
    let title = format!(
        "{SETTINGS_MARK} {}",
        translator.translate(Text::SettingsTitle)
    );
    let mut items = vec![Item::plain(Field::SettingsTitle, vec![value(title)])];
    if !searching.query.is_empty() {
        let query = format!(
            "{QUERY_MARK}{}{QUERY_CARET}{QUERY_COUNT}{}",
            searching.query, searching.matching
        );
        items.push(Item::plain(
            Field::SettingsQuery,
            vec![Span::new(query, LIVE)],
        ));
    }
    let saved = translator.translate(Text::SettingsSaved);
    let saved = Span::new(saved.as_ref(), UNIT).heated(searching.saved_heat);
    items.push(Item::plain(Field::SettingsSaved, vec![saved]));
    let mut keys = Vec::new();
    for (key, text) in KEYS {
        let key = key.map_or_else(|| translator.translate(Text::HelpStopKey), Into::into);
        keys.push(value(format!("{key} ")));
        keys.push(unit(format!("{}  ", translator.translate(text))));
    }
    keys.push(value(SETTINGS_MARK));
    items.push(Item::plain(Field::SettingsKeys, keys).aiming(Target::Settings));
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

fn context(
    status: &SessionStatus,
    (tuning, activity): (&Tuning, &Activity),
    translator: &impl Translator,
) -> Item {
    let tint = activity.gauge_tint;
    let used = status.usage.context_tokens;
    let window = tuning.model.context_window;
    let percent = share(used, window, PERCENT).to_string();

    let mut spans = vec![
        blank(GAUGE_COLUMNS + GAP),
        Span::new(
            translator.with(Text::Percent, &percent),
            tint.map_or(VALUE, Ink::Rgb),
        ),
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
        gauge: Some((
            activity
                .gauge_fill
                .unwrap_or_else(|| share(used, window, PER_MILLE)),
            tint,
        )),
        target: None,
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

fn quiet(text: impl Into<String>) -> Span {
    Span::new(text, GEAR)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/view.rs"]
pub(crate) mod tests;
