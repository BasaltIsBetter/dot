//! Hot paths of the interface, opened to the benches of `benches/` only.
//!
//! Compiled with the `bench` feature, never in the app: each type drives one path the way the
//! window does, without a window or a GPU.

mod agent;
mod command;
mod conversation;
mod keeping;
mod tool;

use std::borrow::Cow;

use dot_api::{
    Effort, Language, Limits, Model, OutputCompaction, Reply, Session, SessionStatus, Spending,
    Summaries, Text, Translator, Tuning, Usage,
};
use std::time::Instant;

use dot_transcript::{rows, to_f32, wrap_characters};

use crate::canvas::{Layout, Scene};
use crate::conversation::Conversation;
use crate::status_bar::{Activity, StatusBar};
use crate::worker::Mailbox;

pub use self::command::CommandBench;
pub use self::conversation::{ConversationBench, ReplyBench, TintBench};
pub use self::keeping::{RecorderBench, ReplayBench};
pub use self::tool::ToolBench;

const SELECTED_REPLY: &str = "A first paragraph long enough to wrap around the column of the conversation at least once, so that its rows end ragged and the selection has to follow them.\n\nA second paragraph, shorter.\n- a list item\n- another list item that goes on\n\nLast words.";
const SELECTION_STARTS: (&str, usize) = ("A first", 8);
const SELECTION_ENDS: (&str, usize) = ("- another", 12);

const TUNING: Tuning = Tuning {
    model: Model::OPUS_5_5,
    effort: Effort::High,
    summaries: Summaries::WhenTheyPay,
    compaction: OutputCompaction::FULL,
};

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
                model_heat: 0,
                effort_heat: 0,
                gauge_tint: None,
                gauge_fill: None,
                name: None,
                name_tint: None,
            },
        }
    }

    /// Makes the strip, as each frame does, and returns how many characters it shows.
    pub fn strip(&mut self) -> usize {
        self.status_bar
            .strip(
                &IdTranslator,
                &self.layout,
                (&self.activity, &TUNING),
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

impl ConversationBench {
    /// Builds a reply of a few paragraphs with a selection over six of its rows, set a while
    /// ago.
    #[must_use]
    pub fn with_selection() -> Self {
        let mut bench = Self::while_selecting();
        bench.conversation.select_release(bench.now);
        bench.now += conversation::SETTLED;
        bench
    }

    /// Builds the same reply while the pointer drags its selection over six of its rows.
    #[must_use]
    pub fn while_selecting() -> Self {
        let layout = Layout::reflowed(conversation::WINDOW, conversation::RETINA);
        let start = Instant::now();
        let mut selected = Conversation::default();
        conversation::send(&mut selected, "explain", start);
        conversation::say(&mut selected, SELECTED_REPLY, start);
        selected.end_reply(None, Some(conversation::TOKENS_OF_A_REPLY), start);
        let now = start + conversation::SETTLED;
        selected.prepare(&layout, now, true);
        let scene = selected.scene(&layout, now, &IdTranslator);
        let from = point_on(&scene, &layout, SELECTION_STARTS);
        let to = point_on(&scene, &layout, SELECTION_ENDS);
        selected.select_press(from, 1, &layout, now);
        selected.select_drag(to, &layout, now);
        Self {
            conversation: selected,
            layout,
            now,
        }
    }
}

fn point_on(scene: &Scene, layout: &Layout, (starts, column): (&str, usize)) -> [f32; 2] {
    let pane = &scene.panes[0];
    let placed = pane
        .lines
        .iter()
        .find(|placed| {
            placed
                .line
                .spans
                .first()
                .is_some_and(|span| span.text.starts_with(starts))
        })
        .expect("the bench's reply shows the rows it selects");
    let columns = f32::from(placed.line.indent) + to_f32(rows(column));
    [
        pane.left + columns * layout.advance() + 1.0,
        placed.top + 1.0,
    ]
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

    fn use_language(&self, _language: Language) {}
}

impl Session for FixedSession {
    fn status(&self) -> SessionStatus {
        SessionStatus {
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
