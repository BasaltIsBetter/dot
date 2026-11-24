use crate::text::Text;

/// Claude model a conversation talks to.
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

/// When a long conversation is summarized, so that Claude stops reading all of it again with
/// every request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Summaries {
    /// Only when the model's context window is nearly full, as Claude decides on its own.
    LeftToClaude,
    /// As soon as what the conversation has already paid to read its surplus again reaches what
    /// a summary costs, measured on the conversation itself.
    WhenTheyPay,
}

/// How Claude answers: what the user may change between two prompts of a conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// Model that answers.
    pub model: Model,
    /// How much it thinks first.
    pub effort: Effort,
    /// When the conversation is summarized.
    pub summaries: Summaries,
}

impl Model {
    /// Claude Haiku 5.5, the fastest and cheapest, with a context window of one million tokens.
    pub const HAIKU_5_5: Self = Self {
        id: "claude-haiku-5-5",
        name: "Haiku 5.5",
        context_window: 1_000_000,
    };

    /// Claude Sonnet 5.5, with a context window of one million tokens.
    pub const SONNET_5_5: Self = Self {
        id: "claude-sonnet-5-5",
        name: "Sonnet 5.5",
        context_window: 1_000_000,
    };

    /// Claude Opus 5.5, Dot's default model, with a context window of one million tokens.
    pub const OPUS_5_5: Self = Self {
        id: "claude-opus-5-5",
        name: "Opus 5.5",
        context_window: 1_000_000,
    };

    /// Claude Fable 5.2, the most capable and the dearest, with a context window of one million
    /// tokens.
    pub const FABLE_5_2: Self = Self {
        id: "claude-fable-5-2",
        name: "Fable 5.2",
        context_window: 1_000_000,
    };

    /// Every model the user can choose, from the cheapest to the most capable.
    pub const ALL: [Self; 4] = [
        Self::HAIKU_5_5,
        Self::SONNET_5_5,
        Self::OPUS_5_5,
        Self::FABLE_5_2,
    ];

    /// Returns the model whose API identifier is `id`, or `None` when no model Dot offers has
    /// it.
    #[must_use]
    pub fn with_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|model| model.id == id)
    }
}

impl Effort {
    /// Every level, from the least thinking to the most.
    pub const ALL: [Self; 5] = [
        Self::Low,
        Self::Medium,
        Self::High,
        Self::ExtraHigh,
        Self::Max,
    ];

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

    /// Returns the level whose name in the Claude API is `name`, such as `high`, or `None`.
    #[must_use]
    pub fn with_api_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|effort| effort.api_name() == name)
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

impl Summaries {
    /// Both choices, Dot's own first.
    pub const ALL: [Self; 2] = [Self::WhenTheyPay, Self::LeftToClaude];

    /// Returns the choice Dot may make when it may make no more than `allowed`: a conversation
    /// whose user set the summaries in Claude Code's own settings is left to Claude, whatever
    /// Dot's setting says.
    #[must_use]
    pub const fn within(self, allowed: Self) -> Self {
        match allowed {
            Self::LeftToClaude => Self::LeftToClaude,
            Self::WhenTheyPay => self,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/settings/tuning.rs"]
mod tests;
