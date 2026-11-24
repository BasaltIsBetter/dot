use dot_api::{
    CaretBlink, Column, Effort, Hue, Language, LiveColor, Model, PermissionMode, ReadingLine,
    SessionsColumn, Settings, Summaries, Text, Toggle, Translator,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    Section(Text),
    Setting(Setting),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Setting {
    Model,
    Effort,
    Summaries,
    Permissions,
    Folder,
    LiveColor,
    StateColors,
    ToolColors,
    SessionColors,
    Language,
    Column,
    ReadingLine,
    Caret,
    SessionsColumn,
    Account,
    ApiKey,
    SignOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Choice,
    Colour,
    Action,
}

pub(crate) const ENTRIES: [Entry; 20] = [
    Entry::Section(Text::SettingsConversation),
    Entry::Setting(Setting::Model),
    Entry::Setting(Setting::Effort),
    Entry::Setting(Setting::Summaries),
    Entry::Setting(Setting::Permissions),
    Entry::Setting(Setting::Folder),
    Entry::Section(Text::SettingsDisplay),
    Entry::Setting(Setting::LiveColor),
    Entry::Setting(Setting::StateColors),
    Entry::Setting(Setting::ToolColors),
    Entry::Setting(Setting::SessionColors),
    Entry::Setting(Setting::Language),
    Entry::Setting(Setting::Column),
    Entry::Setting(Setting::ReadingLine),
    Entry::Setting(Setting::Caret),
    Entry::Setting(Setting::SessionsColumn),
    Entry::Section(Text::SettingsConnection),
    Entry::Setting(Setting::Account),
    Entry::Setting(Setting::ApiKey),
    Entry::Setting(Setting::SignOut),
];

const PERMISSIONS: [PermissionMode; 2] = [PermissionMode::Bypass, PermissionMode::Ask];
const TOGGLES: [Toggle; 2] = [Toggle::On, Toggle::Off];
const COLUMNS: [Column; 3] = [Column::Narrow, Column::Medium, Column::Wide];
const READING_LINES: [ReadingLine; 2] = [ReadingLine::Middle, ReadingLine::Third];
const CARETS: [CaretBlink; 2] = [CaretBlink::Blinking, CaretBlink::Steady];
const SESSIONS_COLUMNS: [SessionsColumn; 2] = [SessionsColumn::Auto, SessionsColumn::Never];

impl Setting {
    pub(crate) const fn label(self) -> Text {
        match self {
            Self::Model => Text::SettingModel,
            Self::Effort => Text::SettingEffort,
            Self::Summaries => Text::SettingSummaries,
            Self::Permissions => Text::SettingPermissions,
            Self::LiveColor => Text::SettingLiveColor,
            Self::StateColors => Text::SettingStateColors,
            Self::ToolColors => Text::SettingToolColors,
            Self::SessionColors => Text::SettingSessionColors,
            Self::Language => Text::SettingLanguage,
            Self::Column => Text::SettingColumn,
            Self::ReadingLine => Text::SettingReadingLine,
            Self::Caret => Text::SettingCaret,
            Self::SessionsColumn => Text::SettingSessionsColumn,
            Self::Folder => Text::SettingFolder,
            Self::Account => Text::SettingAccount,
            Self::ApiKey => Text::SettingApiKey,
            Self::SignOut => Text::SettingSignOut,
        }
    }

    pub(crate) const fn hint(self) -> Text {
        match self {
            Self::Model => Text::SettingModelHint,
            Self::Effort => Text::SettingEffortHint,
            Self::Summaries => Text::SettingSummariesHint,
            Self::Permissions => Text::SettingPermissionsHint,
            Self::LiveColor => Text::SettingLiveColorHint,
            Self::StateColors => Text::SettingStateColorsHint,
            Self::ToolColors => Text::SettingToolColorsHint,
            Self::SessionColors => Text::SettingSessionColorsHint,
            Self::Language => Text::SettingLanguageHint,
            Self::Column => Text::SettingColumnHint,
            Self::ReadingLine => Text::SettingReadingLineHint,
            Self::Caret => Text::SettingCaretHint,
            Self::SessionsColumn => Text::SettingSessionsColumnHint,
            Self::Folder => Text::SettingFolderHint,
            Self::Account => Text::SettingAccountHint,
            Self::ApiKey => Text::SettingApiKeyHint,
            Self::SignOut => Text::SettingSignOutHint,
        }
    }

    pub(crate) const fn kind(self) -> Kind {
        match self {
            Self::LiveColor => Kind::Colour,
            Self::Folder | Self::Account | Self::ApiKey | Self::SignOut => Kind::Action,
            _ => Kind::Choice,
        }
    }

    pub(crate) const fn options(self) -> usize {
        match self {
            Self::Model => Model::ALL.len(),
            Self::Effort => Effort::ALL.len(),
            Self::Summaries => Summaries::ALL.len(),
            Self::Permissions => PERMISSIONS.len(),
            Self::LiveColor => Hue::ALL.len() + 1,
            Self::StateColors | Self::ToolColors | Self::SessionColors => TOGGLES.len(),
            Self::Language => Language::ALL.len(),
            Self::Column => COLUMNS.len(),
            Self::ReadingLine => READING_LINES.len(),
            Self::Caret => CARETS.len(),
            Self::SessionsColumn => SESSIONS_COLUMNS.len(),
            Self::Folder | Self::Account | Self::ApiKey | Self::SignOut => 1,
        }
    }

    pub(crate) fn chosen(self, settings: &Settings) -> usize {
        match self {
            Self::Model => position(&Model::ALL, &settings.tuning.model),
            Self::Effort => position(&Effort::ALL, &settings.tuning.effort),
            Self::Summaries => position(&Summaries::ALL, &settings.tuning.summaries),
            Self::Permissions => position(&PERMISSIONS, &settings.permissions),
            Self::LiveColor => match settings.live_color {
                LiveColor::OfTheSession => 0,
                LiveColor::Fixed(hue) => position(&Hue::ALL, &hue) + 1,
            },
            Self::StateColors => position(&TOGGLES, &settings.state_colors),
            Self::ToolColors => position(&TOGGLES, &settings.tool_colors),
            Self::SessionColors => position(&TOGGLES, &settings.session_colors),
            Self::Language => position(&Language::ALL, &settings.language),
            Self::Column => position(&COLUMNS, &settings.column),
            Self::ReadingLine => position(&READING_LINES, &settings.reading_line),
            Self::Caret => position(&CARETS, &settings.caret),
            Self::SessionsColumn => position(&SESSIONS_COLUMNS, &settings.sessions_column),
            Self::Folder | Self::Account | Self::ApiKey | Self::SignOut => 0,
        }
    }

    pub(crate) fn choose(self, option: usize, settings: &mut Settings) {
        if option >= self.options() {
            return;
        }
        match self {
            Self::Model => settings.tuning.model = Model::ALL[option],
            Self::Effort => settings.tuning.effort = Effort::ALL[option],
            Self::Summaries => settings.tuning.summaries = Summaries::ALL[option],
            Self::Permissions => settings.permissions = PERMISSIONS[option],
            Self::LiveColor => {
                settings.live_color = option
                    .checked_sub(1)
                    .map_or(LiveColor::OfTheSession, |hue| {
                        LiveColor::Fixed(Hue::ALL[hue])
                    });
            }
            Self::StateColors => settings.state_colors = TOGGLES[option],
            Self::ToolColors => settings.tool_colors = TOGGLES[option],
            Self::SessionColors => settings.session_colors = TOGGLES[option],
            Self::Language => settings.language = Language::ALL[option],
            Self::Column => settings.column = COLUMNS[option],
            Self::ReadingLine => settings.reading_line = READING_LINES[option],
            Self::Caret => settings.caret = CARETS[option],
            Self::SessionsColumn => settings.sessions_column = SESSIONS_COLUMNS[option],
            Self::Folder | Self::Account | Self::ApiKey | Self::SignOut => {}
        }
    }

    pub(crate) fn option_name(self, option: usize, translator: &impl Translator) -> String {
        let text = match self {
            Self::Model => {
                return Model::ALL
                    .get(option)
                    .map_or("", |model| model.name)
                    .to_owned();
            }
            Self::Language => {
                return Language::ALL
                    .get(option)
                    .map_or("", |language| language.own_name())
                    .to_owned();
            }
            Self::Column => {
                return COLUMNS.get(option).map_or_default(|column| {
                    let characters = column.characters().to_string();
                    translator
                        .with(Text::ColumnCharacters, &characters)
                        .into_owned()
                });
            }
            Self::Effort => Effort::ALL.get(option).map(|effort| effort.text()),
            Self::Summaries => Summaries::ALL.get(option).map(|summaries| match summaries {
                Summaries::WhenTheyPay => Text::SummariesWhenTheyPay,
                Summaries::LeftToClaude => Text::SummariesLeftToClaude,
            }),
            Self::Permissions => PERMISSIONS.get(option).map(|mode| match mode {
                PermissionMode::Bypass => Text::PermissionsBypass,
                PermissionMode::Ask => Text::PermissionsAsk,
            }),
            Self::LiveColor => match option.checked_sub(1) {
                None => Some(Text::LiveColorOfTheSession),
                Some(hue) => Hue::ALL.get(hue).map(|hue| hue_text(*hue)),
            },
            Self::StateColors | Self::ToolColors | Self::SessionColors => {
                TOGGLES.get(option).map(|toggle| match toggle {
                    Toggle::On => Text::ToggleOn,
                    Toggle::Off => Text::ToggleOff,
                })
            }
            Self::ReadingLine => READING_LINES.get(option).map(|line| match line {
                ReadingLine::Middle => Text::ReadingLineMiddle,
                ReadingLine::Third => Text::ReadingLineThird,
            }),
            Self::Caret => CARETS.get(option).map(|caret| match caret {
                CaretBlink::Blinking => Text::CaretBlinking,
                CaretBlink::Steady => Text::CaretSteady,
            }),
            Self::SessionsColumn => SESSIONS_COLUMNS.get(option).map(|column| match column {
                SessionsColumn::Auto => Text::SessionsColumnAuto,
                SessionsColumn::Never => Text::SessionsColumnNever,
            }),
            Self::Folder | Self::Account | Self::ApiKey | Self::SignOut => None,
        };
        text.map_or_default(|text| translator.translate(text).into_owned())
    }

    pub(crate) fn hue_of(self, option: usize, session: Hue) -> Option<Hue> {
        if self != Self::LiveColor {
            return None;
        }
        option
            .checked_sub(1)
            .map_or(Some(session), |hue| Hue::ALL.get(hue).copied())
    }
}

pub(crate) const fn hue_text(hue: Hue) -> Text {
    match hue {
        Hue::Orange => Text::HueOrange,
        Hue::Red => Text::HueRed,
        Hue::Amber => Text::HueAmber,
        Hue::Green => Text::HueGreen,
        Hue::Cyan => Text::HueCyan,
        Hue::Blue => Text::HueBlue,
        Hue::Violet => Text::HueViolet,
        Hue::Pink => Text::HuePink,
    }
}

fn position<T: PartialEq>(all: &[T], value: &T) -> usize {
    all.iter().position(|each| each == value).unwrap_or(0)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/settings/catalog.rs"]
mod tests;
