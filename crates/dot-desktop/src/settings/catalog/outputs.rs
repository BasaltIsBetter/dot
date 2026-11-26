use dot_api::{CutBounds, CutLevel, OutputCompaction, Text, Toggle, Translator};

const TOGGLES: [Toggle; 2] = [Toggle::On, Toggle::Off];
const BYTES_IN_A_KILOBYTE: usize = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputPart {
    Cleaning,
    Rewriting,
    Progress,
    ContentsCut,
    OutputsCut,
}

impl OutputPart {
    pub(crate) const fn label(self) -> Text {
        match self {
            Self::Cleaning => Text::SettingOutputCleaning,
            Self::Rewriting => Text::SettingOutputRewriting,
            Self::Progress => Text::SettingOutputProgress,
            Self::ContentsCut => Text::SettingContentsCut,
            Self::OutputsCut => Text::SettingOutputsCut,
        }
    }

    pub(crate) const fn hint(self) -> Text {
        match self {
            Self::Cleaning => Text::SettingOutputCleaningHint,
            Self::Rewriting => Text::SettingOutputRewritingHint,
            Self::Progress => Text::SettingOutputProgressHint,
            Self::ContentsCut => Text::SettingContentsCutHint,
            Self::OutputsCut => Text::SettingOutputsCutHint,
        }
    }

    pub(crate) const fn options(self) -> usize {
        match self {
            Self::Cleaning | Self::Rewriting | Self::Progress => TOGGLES.len(),
            Self::ContentsCut | Self::OutputsCut => CutLevel::ALL.len(),
        }
    }

    pub(crate) fn chosen(self, compaction: OutputCompaction) -> usize {
        match self {
            Self::Cleaning => toggle_position(compaction.cleaning),
            Self::Rewriting => toggle_position(compaction.rewriting),
            Self::Progress => toggle_position(compaction.progress),
            Self::ContentsCut => level_position(compaction.contents_cut),
            Self::OutputsCut => level_position(compaction.outputs_cut),
        }
    }

    pub(crate) fn choose(self, option: usize, compaction: &mut OutputCompaction) {
        match self {
            Self::Cleaning => set(&TOGGLES, option, &mut compaction.cleaning),
            Self::Rewriting => set(&TOGGLES, option, &mut compaction.rewriting),
            Self::Progress => set(&TOGGLES, option, &mut compaction.progress),
            Self::ContentsCut => set(&CutLevel::ALL, option, &mut compaction.contents_cut),
            Self::OutputsCut => set(&CutLevel::ALL, option, &mut compaction.outputs_cut),
        }
    }

    pub(crate) fn option_name(self, option: usize, translator: &impl Translator) -> String {
        match self {
            Self::Cleaning | Self::Rewriting | Self::Progress => {
                TOGGLES.get(option).map_or_default(|toggle| {
                    let text = match toggle {
                        Toggle::On => Text::ToggleOn,
                        Toggle::Off => Text::ToggleOff,
                    };
                    translator.translate(text).into_owned()
                })
            }
            Self::ContentsCut => cut_name(option, CutLevel::of_contents, translator),
            Self::OutputsCut => cut_name(option, CutLevel::of_outputs, translator),
        }
    }
}

fn cut_name(
    option: usize,
    bounds_of: fn(CutLevel) -> Option<CutBounds>,
    translator: &impl Translator,
) -> String {
    let Some(&level) = CutLevel::ALL.get(option) else {
        return String::new();
    };
    let Some(bounds) = bounds_of(level) else {
        return translator.translate(Text::CutNever).into_owned();
    };
    let kilobytes = format!(
        "{} → {}",
        bounds.above / BYTES_IN_A_KILOBYTE,
        bounds.kept / BYTES_IN_A_KILOBYTE
    );
    translator.with(Text::CutBounds, &kilobytes).into_owned()
}

fn toggle_position(toggle: Toggle) -> usize {
    TOGGLES.iter().position(|each| *each == toggle).unwrap_or(0)
}

fn level_position(level: CutLevel) -> usize {
    CutLevel::ALL
        .iter()
        .position(|each| *each == level)
        .unwrap_or(0)
}

fn set<T: Copy>(all: &[T], option: usize, value: &mut T) {
    if let Some(&chosen) = all.get(option) {
        *value = chosen;
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/settings/catalog/outputs.rs"]
mod tests;
