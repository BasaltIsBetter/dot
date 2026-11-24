use dot_api::{
    CaretBlink, Column, Effort, Hue, Language, LiveColor, Model, PermissionMode, ReadingLine,
    SessionsColumn, Settings, Summaries, Toggle,
};
use serde_json::{Map, Value};

const OF_THE_SESSION: &str = "session";

const SUMMARIES: [(Summaries, &str); 2] = [
    (Summaries::WhenTheyPay, "when-they-pay"),
    (Summaries::LeftToClaude, "left-to-claude"),
];
const PERMISSIONS: [(PermissionMode, &str); 2] = [
    (PermissionMode::Bypass, "bypass"),
    (PermissionMode::Ask, "ask"),
];
const HUES: [(Hue, &str); 8] = [
    (Hue::Orange, "orange"),
    (Hue::Red, "red"),
    (Hue::Amber, "amber"),
    (Hue::Green, "green"),
    (Hue::Cyan, "cyan"),
    (Hue::Blue, "blue"),
    (Hue::Violet, "violet"),
    (Hue::Pink, "pink"),
];
const TOGGLES: [(Toggle, &str); 2] = [(Toggle::On, "on"), (Toggle::Off, "off")];
const COLUMNS: [(Column, &str); 3] = [
    (Column::Narrow, "64"),
    (Column::Medium, "80"),
    (Column::Wide, "96"),
];
const READING_LINES: [(ReadingLine, &str); 2] = [
    (ReadingLine::Middle, "middle"),
    (ReadingLine::Third, "third"),
];
const CARETS: [(CaretBlink, &str); 2] = [
    (CaretBlink::Blinking, "blinking"),
    (CaretBlink::Steady, "steady"),
];
const SESSIONS_COLUMNS: [(SessionsColumn, &str); 2] = [
    (SessionsColumn::Auto, "auto"),
    (SessionsColumn::Never, "never"),
];

struct Field {
    key: &'static str,
    name: fn(&Settings) -> &'static str,
    set: fn(&mut Settings, &str) -> Option<()>,
}

const FIELDS: [Field; 13] = [
    Field {
        key: "model",
        name: |settings| settings.tuning.model.id,
        set: |settings, name| {
            settings.tuning.model = Model::with_id(name)?;
            Some(())
        },
    },
    Field {
        key: "effort",
        name: |settings| settings.tuning.effort.api_name(),
        set: |settings, name| {
            settings.tuning.effort = Effort::with_api_name(name)?;
            Some(())
        },
    },
    Field {
        key: "summaries",
        name: |settings| named(&SUMMARIES, settings.tuning.summaries),
        set: |settings, name| {
            settings.tuning.summaries = by_name(&SUMMARIES, name)?;
            Some(())
        },
    },
    Field {
        key: "actions",
        name: |settings| named(&PERMISSIONS, settings.permissions),
        set: |settings, name| {
            settings.permissions = by_name(&PERMISSIONS, name)?;
            Some(())
        },
    },
    Field {
        key: "live-color",
        name: |settings| match settings.live_color {
            LiveColor::OfTheSession => OF_THE_SESSION,
            LiveColor::Fixed(hue) => named(&HUES, hue),
        },
        set: |settings, name| {
            settings.live_color = if name == OF_THE_SESSION {
                LiveColor::OfTheSession
            } else {
                LiveColor::Fixed(by_name(&HUES, name)?)
            };
            Some(())
        },
    },
    Field {
        key: "state-colors",
        name: |settings| named(&TOGGLES, settings.state_colors),
        set: |settings, name| {
            settings.state_colors = by_name(&TOGGLES, name)?;
            Some(())
        },
    },
    Field {
        key: "tool-colors",
        name: |settings| named(&TOGGLES, settings.tool_colors),
        set: |settings, name| {
            settings.tool_colors = by_name(&TOGGLES, name)?;
            Some(())
        },
    },
    Field {
        key: "session-colors",
        name: |settings| named(&TOGGLES, settings.session_colors),
        set: |settings, name| {
            settings.session_colors = by_name(&TOGGLES, name)?;
            Some(())
        },
    },
    Field {
        key: "language",
        name: |settings| settings.language.code(),
        set: |settings, name| {
            settings.language = Language::with_code(name)?;
            Some(())
        },
    },
    Field {
        key: "column",
        name: |settings| named(&COLUMNS, settings.column),
        set: |settings, name| {
            settings.column = by_name(&COLUMNS, name)?;
            Some(())
        },
    },
    Field {
        key: "reading-line",
        name: |settings| named(&READING_LINES, settings.reading_line),
        set: |settings, name| {
            settings.reading_line = by_name(&READING_LINES, name)?;
            Some(())
        },
    },
    Field {
        key: "caret",
        name: |settings| named(&CARETS, settings.caret),
        set: |settings, name| {
            settings.caret = by_name(&CARETS, name)?;
            Some(())
        },
    },
    Field {
        key: "sessions-column",
        name: |settings| named(&SESSIONS_COLUMNS, settings.sessions_column),
        set: |settings, name| {
            settings.sessions_column = by_name(&SESSIONS_COLUMNS, name)?;
            Some(())
        },
    },
];

pub(crate) fn read(written: &[u8], defaults: &Settings) -> Settings {
    let mut settings = *defaults;
    let Ok(Value::Object(fields)) = serde_json::from_slice::<Value>(written) else {
        return settings;
    };
    for field in &FIELDS {
        if let Some(name) = fields.get(field.key).and_then(Value::as_str) {
            let mut changed = settings;
            if (field.set)(&mut changed, name).is_some() {
                settings = changed;
            }
        }
    }
    settings
}

pub(crate) fn written(settings: &Settings, defaults: &Settings) -> Vec<u8> {
    let fields: Map<String, Value> = FIELDS
        .iter()
        .map(|field| (field.key, (field.name)(settings), (field.name)(defaults)))
        .filter(|(_, name, default)| name != default)
        .map(|(key, name, _)| (key.to_owned(), Value::from(name)))
        .collect();
    let mut bytes = Value::Object(fields).to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

fn named<T: Copy + PartialEq>(table: &[(T, &'static str)], value: T) -> &'static str {
    table
        .iter()
        .find(|(each, _)| *each == value)
        .map_or("", |(_, name)| name)
}

fn by_name<T: Copy>(table: &[(T, &'static str)], name: &str) -> Option<T> {
    table
        .iter()
        .find(|(_, each)| *each == name)
        .map(|(value, _)| *value)
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/settings/stored.rs"]
pub(crate) mod tests;
