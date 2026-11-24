use crate::chat::Reply;

/// How the project's own instructions, its `CLAUDE.md`, reach Claude through Claude Code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instructions {
    /// Claude Code loads the file itself, with the conversation: it is written to the cache
    /// again after every summary and at the start of every conversation.
    LoadedByClaudeCode,
    /// Dot hands the file over word for word in the system prompt, where it stays cached
    /// across summaries and conversations, keeps Claude Code from loading it a second time,
    /// and tells each sub-agent to read it. A file Dot cannot hand over safely is left to
    /// Claude Code.
    HandedByDot,
}

/// Who reads the project when Claude only has to find or understand something in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Claude itself: what it reads enters the conversation and is read again with every later
    /// request.
    ByClaude,
    /// An agent of Dot's on a smaller, cheaper model, which reads and reports: what it read
    /// never enters the conversation. Claude decides when to hand the reading over.
    ByACheaperAgent,
}

/// Amount of money in US dollars, counted in millionths of a dollar so that sums stay exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cost(u64);

/// What a session has consumed since it opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Tokens the model read on its latest turn: how full its context window is.
    pub context_tokens: u64,
    /// Tokens the model read on the first request of the conversation, before anything was
    /// said; zero until a reply has started the conversation.
    pub base_tokens: u64,
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

/// What a session has consumed so far, with the plan's limits: everything about it that is
/// kept in memory, never read from the disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spending {
    /// What the session has consumed.
    pub usage: Usage,
    /// Share of the plan's usage limits already spent.
    pub limits: Limits,
}

/// Where a session stands, as the interface shows it.
///
/// The model and the effort are not part of it: they are the user's
/// [`Settings`](crate::Settings), the same for every session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionStatus {
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
///
/// The session keeps what its replies consume, whichever thread they run on, so that a reply
/// stopped halfway still counts.
pub trait Session: Send + Sync {
    /// Returns where the session stands now, what it has consumed included.
    ///
    /// May read the session's folder, for its git branch: call it off the interface thread. A
    /// folder or a branch that cannot be read is reported as absent.
    fn status(&self) -> SessionStatus;

    /// Returns what the session has consumed so far, without touching the disk.
    fn spending(&self) -> Spending;

    /// Adds what a reply consumed, whether Claude finished it or not: its cost to the total,
    /// its context in place of the previous one when the reply reached Claude, the base of the
    /// conversation when the reply started one, and the plan's limits it reported.
    fn record(&self, reply: &Reply);

    /// Adds the lines Claude's tools added to and removed from the project's files.
    fn count_changes(&self, added: u64, removed: u64);

    /// Empties the context and forgets its base: the conversation starts afresh. What was spent
    /// stays counted.
    fn clear_context(&self);
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
