//! Hot paths of the interface, opened to the benches of `benches/` only.
//!
//! Compiled with the `bench` feature, never in the app: each type drives one path the way the
//! window does, without a window or a GPU.

mod conversation;
mod tool;

use std::borrow::Cow;

use dot_api::{
    Effort, Limits, Model, Reply, Session, SessionStatus, Spending, Text, Translator, Usage,
};
use dot_transcript::wrap_characters;

use crate::canvas::Layout;
use crate::status_bar::{Activity, StatusBar};
use crate::worker::Mailbox;

pub use self::conversation::{ConversationBench, ReplyBench};
pub use self::tool::ToolBench;

/// The status bar of a session, made into its strip like at every frame.
pub struct StripBench {
    status_bar: StatusBar<NoMail>,
    layout: Layout,
    activity: Activity,
}

struct IdTranslator;

struct FixedSession;

#[derive(Clone)]
struct NoMail;

impl StripBench {
    /// Prepares the status bar of a session that has spent a little.
    #[must_use]
    pub fn new() -> Self {
        let mut status_bar = StatusBar::new(NoMail);
        status_bar.report(FixedSession.status());
        Self {
            status_bar,
            layout: Layout::reflowed(conversation::WINDOW, conversation::RETINA),
            activity: Activity {
                text: Some("state-writing".to_owned()),
                live: true,
                sessions: None,
            },
        }
    }

    /// Makes the strip, as each frame does, and returns how many characters it shows.
    pub fn strip(&mut self) -> usize {
        self.status_bar
            .strip(
                &IdTranslator,
                &self.layout,
                &self.activity,
                FixedSession.spending(),
            )
            .map_or(0, |strip| strip.text.spans.len())
    }
}

impl Default for StripBench {
    fn default() -> Self {
        Self::new()
    }
}

/// Cuts `text` into the rows of a draft `columns` cells wide and returns how many there are.
#[must_use]
pub fn cut_draft(text: &str, columns: usize) -> usize {
    wrap_characters(text, columns).len()
}

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }

    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
    }
}

impl Session for FixedSession {
    fn status(&self) -> SessionStatus {
        SessionStatus {
            model: Model::OPUS_5_5,
            effort: Effort::High,
            project: Some("Dot".to_owned()),
            branch: Some("feat/mac-app".to_owned()),
            usage: Usage::default(),
            limits: Limits::default(),
        }
    }

    fn spending(&self) -> Spending {
        Spending {
            usage: Usage {
                context_tokens: 147_000,
                base_tokens: 0,
                lines_added: 11,
                lines_removed: 48,
                cost: dot_api::Cost::from_micro_dollars(2_400_000),
            },
            limits: Limits::default(),
        }
    }

    fn record(&self, _reply: &Reply) {}

    fn count_changes(&self, _added: u64, _removed: u64) {}

    fn clear_context(&self) {}
}

impl<E> Mailbox<E> for NoMail {
    fn deliver(&self, _event: E) {}
}
