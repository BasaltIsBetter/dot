use std::borrow::Cow;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::chat::{Reply, ReplyEvent};
use crate::session::Cost;
use crate::text::Text;

/// One thing that happened in the conversation of a session, at a time on the session's own
/// clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<'a> {
    /// When it happened, counted from the start of the session: the time the session was open,
    /// its launches put end to end with a short pause between two.
    pub at: Duration,
    /// What happened.
    pub happened: Happened<'a>,
}

/// What happened in the conversation of a session, kept so that it can be shown again after Dot
/// restarts.
///
/// Kept in the journal of the session under the names its serialization gives: they stay the
/// same from one version of Dot to the next. What is borrowed is written without a copy; what is
/// read back is owned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Happened<'a> {
    /// The user sent Claude something.
    Asked(Asked<'a>),
    /// A piece of Claude's reply arrived.
    Heard(Cow<'a, ReplyEvent>),
    /// The reply ended, well or not.
    Ended(Ending),
    /// The user started the conversation afresh: Claude forgot what was said before.
    StartedOver,
    /// What the session had spent in the part of its journal that Dot no longer keeps.
    Carried(Carried),
}

/// What the user sent Claude, as the conversation shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asked<'a> {
    /// The text sent; empty when only images were.
    pub text: Cow<'a, str>,
    /// How it was sent.
    pub opening: Opening,
    /// The images sent with it, in the order they were attached; the images themselves are not
    /// kept.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageLabel>,
}

/// How something was sent to Claude.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Opening {
    /// A prompt, while Claude was not answering.
    Prompt,
    /// A note written while the reply was paused, sent once that reply had ended.
    Note,
    /// A note written while the reply was paused, sent in place of the rest of that reply: the
    /// reply ended there, without a word.
    Redirect,
}

/// An image sent to Claude, as the conversation names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageLabel {
    /// The name of the image's file, or the name it was given when pasted.
    pub name: String,
    /// Its size, in bytes.
    pub bytes: u64,
}

/// How a reply ended.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ending {
    /// What the reply consumed, whether Claude finished it or not.
    pub spent: Reply,
    /// Why the reply did not end as Claude finished it; `None` when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
}

/// Why a reply did not end as Claude finished it, as the conversation told the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    /// The text that told the user.
    pub text: Text,
    /// The technical cause shown under it, in English, when there was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}

/// What a session spent in the part of its journal that Dot no longer keeps, so that the totals
/// stay right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Carried {
    /// What the replies cost.
    pub cost: Cost,
    /// Lines Claude's tools added to files.
    pub lines_added: u64,
    /// Lines Claude's tools removed from files.
    pub lines_removed: u64,
}

impl Happened<'_> {
    /// Returns the same thing owning all of its parts, free of what it borrowed.
    #[must_use]
    pub fn into_owned(self) -> Happened<'static> {
        match self {
            Self::Asked(asked) => Happened::Asked(Asked {
                text: Cow::Owned(asked.text.into_owned()),
                opening: asked.opening,
                images: asked.images,
            }),
            Self::Heard(event) => Happened::Heard(Cow::Owned(event.into_owned())),
            Self::Ended(ending) => Happened::Ended(ending),
            Self::StartedOver => Happened::StartedOver,
            Self::Carried(carried) => Happened::Carried(carried),
        }
    }

    /// Returns whether a conversation shown from here on reads as a whole: what the user sent,
    /// or a fresh start. A journal cut short starts at one of these.
    #[must_use]
    pub const fn opens_an_exchange(&self) -> bool {
        matches!(self, Self::Asked(_) | Self::StartedOver)
    }
}

impl Carried {
    /// Adds what `happened` spent: the cost of an ending, the lines of a tool that changed files,
    /// another part already carried.
    pub fn add(&mut self, happened: &Happened<'_>) {
        match happened {
            Happened::Ended(ending) => {
                self.cost = Cost::from_micro_dollars(
                    self.cost
                        .micro_dollars()
                        .saturating_add(ending.spent.cost.micro_dollars()),
                );
            }
            Happened::Heard(event) => {
                if let Some((added, removed)) = changed_lines(event) {
                    self.lines_added = self.lines_added.saturating_add(added);
                    self.lines_removed = self.lines_removed.saturating_add(removed);
                }
            }
            Happened::Carried(carried) => {
                self.cost = Cost::from_micro_dollars(
                    self.cost
                        .micro_dollars()
                        .saturating_add(carried.cost.micro_dollars()),
                );
                self.lines_added = self.lines_added.saturating_add(carried.lines_added);
                self.lines_removed = self.lines_removed.saturating_add(carried.lines_removed);
            }
            Happened::Asked(_) | Happened::StartedOver => {}
        }
    }
}

/// Returns the lines added and removed by the tool whose end `event` tells, when that tool
/// changed files.
#[must_use]
pub const fn changed_lines(event: &ReplyEvent) -> Option<(u64, u64)> {
    match event {
        ReplyEvent::ToolEnded {
            end: crate::tool::ToolEnd::Done(crate::tool::Detail::Changes { added, removed }),
            ..
        } => Some((*added, *removed)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-api/archive/record.rs"]
mod tests;
