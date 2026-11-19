//! Hot paths of the interface, opened to the benches of `benches/` only.
//!
//! Compiled with the `bench` feature, never in the app: each type drives one path the way the
//! window does, without a window or a GPU.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dot_api::{
    Effort, Limits, Model, Reply, ReplyEvent, Session, SessionStatus, Spending, Text, Translator,
    Usage,
};
use winit::dpi::PhysicalSize;

use crate::canvas::Layout;
use crate::conversation::{Conversation, Edit, Intent, wrap_characters};
use crate::status_bar::StatusBar;
use crate::worker::Mailbox;

const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2_000, 1_400);
const RETINA: f64 = 2.0;
const STRIP_COLUMNS: u16 = 160;
const SETTLED: Duration = Duration::from_secs(5);

/// A conversation with a long history, composed into frames like the window composes them.
pub struct ConversationBench {
    conversation: Conversation,
    layout: Layout,
    now: Instant,
}

/// A reply that keeps growing, piece by piece, in a conversation.
pub struct ReplyBench {
    conversation: Conversation,
    layout: Layout,
    now: Instant,
}

/// The status bar of a session, made into its strip like at every frame.
pub struct StripBench {
    status_bar: StatusBar<FixedSession, NoMail>,
}

struct IdTranslator;

struct FixedSession;

#[derive(Clone)]
struct NoMail;

impl ConversationBench {
    /// Builds a conversation of `exchanges` short prompts, each with its reply, the view at the
    /// end and every animation over.
    #[must_use]
    pub fn with_history(exchanges: usize) -> Self {
        let layout = Layout::reflowed(WINDOW, RETINA);
        let start = Instant::now();
        let mut conversation = Conversation::default();
        for exchange in 0..exchanges {
            send(
                &mut conversation,
                &format!("question {exchange}"),
                &layout,
                start,
            );
            let reply = ReplyEvent::Text(format!("answer {exchange} with a few words"));
            conversation.hear(reply, &layout, start, &IdTranslator);
            conversation.end_reply(None, &layout, start);
        }
        Self {
            conversation,
            layout,
            now: start + SETTLED,
        }
    }

    /// Composes one frame and returns how many lines its panes show.
    pub fn frame(&mut self) -> usize {
        self.conversation
            .prepare(&self.layout, self.now, &IdTranslator);
        let scene = self
            .conversation
            .scene(&self.layout, self.now, true, &IdTranslator);
        scene.panes.iter().map(|pane| pane.lines.len()).sum()
    }
}

impl ReplyBench {
    /// Starts a reply already `length` bytes long, made of short words and line breaks.
    #[must_use]
    pub fn of_length(length: usize) -> Self {
        let layout = Layout::reflowed(WINDOW, RETINA);
        let now = Instant::now();
        let mut conversation = Conversation::default();
        send(&mut conversation, "write a long answer", &layout, now);
        let text = "word ".repeat(length / 5);
        conversation.hear(ReplyEvent::Text(text), &layout, now, &IdTranslator);
        Self {
            conversation,
            layout,
            now,
        }
    }

    /// Hands the reply one more piece, as it arrives from Claude.
    pub fn hear(&mut self, piece: &str) {
        self.now += Duration::from_millis(1);
        let event = ReplyEvent::Text(piece.to_owned());
        self.conversation
            .hear(event, &self.layout, self.now, &IdTranslator);
    }
}

impl StripBench {
    /// Prepares the status bar of a session that has spent a little.
    #[must_use]
    pub fn new() -> Self {
        let mut status_bar = StatusBar::new(Arc::new(FixedSession), NoMail);
        status_bar.report(FixedSession.status());
        Self { status_bar }
    }

    /// Makes the strip, as each frame does, and returns how many characters it shows.
    pub fn strip(&mut self) -> usize {
        self.status_bar
            .strip(&IdTranslator, STRIP_COLUMNS)
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

fn send(conversation: &mut Conversation, text: &str, layout: &Layout, now: Instant) {
    let insert = Intent::Edit(Edit::Insert(text.to_owned()));
    conversation.type_key(insert, layout, now, || None);
    conversation.type_key(Intent::Send, layout, now, || None);
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
