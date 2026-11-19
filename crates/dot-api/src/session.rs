use crate::text::Text;

/// Claude model a session talks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    /// Identifier sent to the Claude API, such as `claude-opus-5-5`.
    pub id: &'static str,
    /// Short name shown to the user, such as `Opus 5.5`.
    pub name: &'static str,
    /// Most tokens the model reads at once: the size of its context window.
    pub context_window: u64,
}

/// How much Claude thinks before it answers: more effort, deeper answers, slower and dearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    /// Short thinking, for quick questions.
    Low,
    /// The model's own default.
    Medium,
    /// Deeper thinking, Dot's default.
    High,
    /// Deeper still, for hard coding work.
    ExtraHigh,
    /// No limit on thinking.
    Max,
}

/// Amount of money in US dollars, counted in millionths of a dollar so that sums stay exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cost(u64);

/// What a session has consumed since it opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Tokens the model read on its latest turn: how full its context window is.
    pub context_tokens: u64,
    /// Lines the session added to the project's files.
    pub lines_added: u64,
    /// Lines the session removed from the project's files.
    pub lines_removed: u64,
    /// What the session's requests to Claude cost.
    pub cost: Cost,
}

/// Share of the Claude plan's usage limits already spent, in percent.
///
/// `None` while Dot has not received the value: only a Claude plan has these limits, and Claude
/// reports them with its answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// Limit over a rolling window of 5 hours.
    pub five_hours: Option<u8>,
    /// Limit over a rolling window of 7 days.
    pub seven_days: Option<u8>,
}

/// Where a session stands, as the interface shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStatus {
    /// Model the session talks to.
    pub model: Model,
    /// How much the model thinks before it answers.
    pub effort: Effort,
    /// Name of the session's folder, or `None` when the session has no folder.
    pub project: Option<String>,
    /// Git branch checked out in the folder, or the short hash of the commit when no branch is;
    /// `None` when the folder is not the root of a git repository.
    pub branch: Option<String>,
    /// What the session has consumed so far.
    pub usage: Usage,
    /// Share of the plan's usage limits already spent.
    pub limits: Limits,
}

/// A Dot session: the work of one window with Claude, in one folder.
pub trait Session: Send + Sync {
    /// Returns where the session stands now.
    ///
    /// May read the session's folder, for its git branch: call it off the interface thread. A
    /// folder or a branch that cannot be read is reported as absent.
    fn status(&self) -> SessionStatus;
}

impl Model {
    /// Claude Opus 5.5, Dot's default model, with a context window of one million tokens.
    pub const OPUS_5_5: Self = Self {
        id: "claude-opus-5-5",
        name: "Opus 5.5",
        context_window: 1_000_000,
    };
}

impl Effort {
    /// Returns the level's name in the Claude API and in Claude Code, such as `high`.
    #[must_use]
    pub const fn api_name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::ExtraHigh => "xhigh",
            Self::Max => "max",
        }
    }

    /// Returns the text that names the level to the user.
    #[must_use]
    pub const fn text(self) -> Text {
        match self {
            Self::Low => Text::EffortLow,
            Self::Medium => Text::EffortMedium,
            Self::High => Text::EffortHigh,
            Self::ExtraHigh => Text::EffortExtraHigh,
            Self::Max => Text::EffortMax,
        }
    }
}

impl Cost {
    /// Creates a cost of `micro_dollars` millionths of a US dollar.
    #[must_use]
    pub const fn from_micro_dollars(micro_dollars: u64) -> Self {
        Self(micro_dollars)
    }

    /// Returns the cost in millionths of a US dollar.
    #[must_use]
    pub const fn micro_dollars(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-api/session.rs"]
mod tests;
