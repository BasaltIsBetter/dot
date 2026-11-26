use crate::settings::look::Toggle;

const PAIR_SEPARATOR: char = ',';
const KEY_SEPARATOR: char = '=';
const CLEANING_KEY: &str = "cleaning";
const REWRITING_KEY: &str = "rewriting";
const PROGRESS_KEY: &str = "progress";
const CONTENTS_KEY: &str = "contents";
const OUTPUTS_KEY: &str = "outputs";
const ON: &str = "on";
const OFF: &str = "off";
const NEVER: &str = "never";
const LATE: &str = "late";
const MEASURED: &str = "measured";
const EARLY: &str = "early";
const FIELD_COUNT: usize = 5;

/// Name of the variable through which Dot hands its [`OutputCompaction`] to its stand-in for
/// the shell of Claude Code's commands, written by [`OutputCompaction::to_variable`].
pub const COMPACTION_VARIABLE: &str = "DOT_COMPACTION";

/// How much of a long output Claude still reads: when it is cut, and how much is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutLevel {
    /// Never cut: Claude reads it whole.
    Never,
    /// Cut later and keep more than Dot's measured bounds.
    Late,
    /// The bounds Dot measured on real conversations, its default.
    Measured,
    /// Cut sooner and keep less than Dot's measured bounds.
    Early,
}

/// Where a long output is cut, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutBounds {
    /// An output longer than this is cut.
    pub above: usize,
    /// About how much of it Claude still reads once it is cut.
    pub kept: usize,
}

/// What Dot does to shorten what Claude reads from commands and searches, each part on or off.
///
/// Whatever is on, the contents of a file and a diff are never rewritten, and a command that
/// starts with [`RAW_OUTPUT_MARK`](crate::RAW_OUTPUT_MARK) keeps its output whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputCompaction {
    /// The colours of the terminal, and the lines a program redraws in place, told once.
    pub cleaning: Toggle,
    /// Rewritings that lose nothing: repeated paths written once, lists of files folded,
    /// indented JSON without its spaces, trees, tables, `git status` and `git log` shorter.
    pub rewriting: Toggle,
    /// The progress lines of the tools a command names, and the tests that passed, counted.
    pub progress: Toggle,
    /// When the contents of a file and a diff are cut.
    pub contents_cut: CutLevel,
    /// When every other output of a command is cut.
    pub outputs_cut: CutLevel,
}

impl CutLevel {
    /// Every level, from the one that cuts least to the one that cuts most.
    pub const ALL: [Self; 4] = [Self::Never, Self::Late, Self::Measured, Self::Early];

    /// Returns where the contents of a file and a diff are cut, `None` when they never are.
    #[must_use]
    pub const fn of_contents(self) -> Option<CutBounds> {
        match self {
            Self::Never => None,
            Self::Late => Some(CutBounds::new(13_000, 9_000)),
            Self::Measured => Some(CutBounds::new(10_000, 6_000)),
            Self::Early => Some(CutBounds::new(8_000, 4_000)),
        }
    }

    /// Returns where any other output of a command is cut, `None` when it never is.
    #[must_use]
    pub const fn of_outputs(self) -> Option<CutBounds> {
        match self {
            Self::Never => None,
            Self::Late => Some(CutBounds::new(9_000, 5_000)),
            Self::Measured => Some(CutBounds::new(7_000, 3_000)),
            Self::Early => Some(CutBounds::new(6_000, 2_000)),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Never => NEVER,
            Self::Late => LATE,
            Self::Measured => MEASURED,
            Self::Early => EARLY,
        }
    }

    fn with_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == name)
    }
}

impl CutBounds {
    const fn new(above: usize, kept: usize) -> Self {
        Self { above, kept }
    }
}

impl OutputCompaction {
    /// Every part on, and long outputs cut at the bounds Dot measured.
    pub const FULL: Self = Self {
        cleaning: Toggle::On,
        rewriting: Toggle::On,
        progress: Toggle::On,
        contents_cut: CutLevel::Measured,
        outputs_cut: CutLevel::Measured,
    };

    /// Returns the value of [`COMPACTION_VARIABLE`] that tells these choices, such as
    /// `cleaning=on,rewriting=on,progress=off,contents=measured,outputs=never`.
    #[must_use]
    pub fn to_variable(&self) -> String {
        let toggle = |toggle: Toggle| if toggle.is_on() { ON } else { OFF };
        format!(
            "{CLEANING_KEY}={},{REWRITING_KEY}={},{PROGRESS_KEY}={},{CONTENTS_KEY}={},{OUTPUTS_KEY}={}",
            toggle(self.cleaning),
            toggle(self.rewriting),
            toggle(self.progress),
            self.contents_cut.name(),
            self.outputs_cut.name(),
        )
    }

    /// Returns the choices that `value` tells, as [`OutputCompaction::to_variable`] writes
    /// them, or `None` unless each of the five is there exactly once, with a known value.
    #[must_use]
    pub fn from_variable(value: &str) -> Option<Self> {
        let mut read = [None::<&str>; FIELD_COUNT];
        for pair in value.split(PAIR_SEPARATOR) {
            let (key, value) = pair.split_once(KEY_SEPARATOR)?;
            let place = [
                CLEANING_KEY,
                REWRITING_KEY,
                PROGRESS_KEY,
                CONTENTS_KEY,
                OUTPUTS_KEY,
            ]
            .iter()
            .position(|known| *known == key)?;
            if read[place].replace(value).is_some() {
                return None;
            }
        }
        let [
            Some(cleaning),
            Some(rewriting),
            Some(progress),
            Some(contents),
            Some(outputs),
        ] = read
        else {
            return None;
        };
        Some(Self {
            cleaning: toggle_named(cleaning)?,
            rewriting: toggle_named(rewriting)?,
            progress: toggle_named(progress)?,
            contents_cut: CutLevel::with_name(contents)?,
            outputs_cut: CutLevel::with_name(outputs)?,
        })
    }
}

fn toggle_named(name: &str) -> Option<Toggle> {
    match name {
        ON => Some(Toggle::On),
        OFF => Some(Toggle::Off),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/settings/outputs.rs"]
mod tests;
