use std::time::{Duration, Instant};

use dot_api::{Detail, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::dpi::PhysicalSize;

use crate::bench::IdTranslator;
use crate::canvas::{Layout, Pane};
use crate::conversation::{Conversation, Edit, Intent, Tints, tint};

pub(super) const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2_000, 1_400);
const FRAME: Duration = Duration::from_millis(16);
pub(super) const RETINA: f64 = 2.0;
pub(super) const SETTLED: Duration = Duration::from_secs(5);
pub(super) const ARRIVING: Duration = Duration::from_millis(100);
const TOOL_SUBJECT: &str = "crates/dot-desktop/src/conversation/transcript/history/tool.rs";
const TOOL_LINES: u64 = 227;
pub(super) const TOKENS_OF_A_REPLY: u64 = 12;

/// A conversation with a long history, composed into frames like the window composes them.
pub struct ConversationBench {
    pub(super) conversation: Conversation,
    pub(super) layout: Layout,
    pub(super) now: Instant,
}

/// The lines of a frame full of tools, before the colours of meaning are laid on them.
pub struct TintBench {
    panes: Vec<Pane>,
}

const SESSION_COLOUR: [u8; 3] = [0xa9, 0x70, 0xff];
const STOPPED: &str = "stopped";

/// A reply that keeps growing, piece by piece, in a conversation.
pub struct ReplyBench {
    conversation: Conversation,
    now: Instant,
}

impl ConversationBench {
    /// Builds a conversation of `exchanges` short prompts, each with its reply, the view at the
    /// end and every animation over.
    #[must_use]
    pub fn with_history(exchanges: usize) -> Self {
        let layout = Layout::reflowed(WINDOW, RETINA);
        let start = Instant::now();
        let mut conversation = Conversation::default();
        for exchange in 0..exchanges {
            send(&mut conversation, &format!("question {exchange}"), start);
            say(
                &mut conversation,
                &format!("answer {exchange} with a few words"),
                start,
            );
            conversation.end_reply(None, Some(TOKENS_OF_A_REPLY), start);
        }
        Self {
            conversation,
            layout,
            now: start + SETTLED,
        }
    }

    /// Builds a conversation of `exchanges` prompts whose replies each say a few words, run
    /// `tools` tools in a row and say a few more words, the view at the end and every animation
    /// over.
    #[must_use]
    pub fn with_tools(exchanges: usize, tools: usize) -> Self {
        let layout = Layout::reflowed(WINDOW, RETINA);
        let start = Instant::now();
        let mut conversation = Conversation::default();
        for exchange in 0..exchanges {
            send(&mut conversation, &format!("question {exchange}"), start);
            say(&mut conversation, "Let me look.", start);
            for tool in 0..tools {
                let id = format!("{exchange}-{tool}");
                start_tool(&mut conversation, &id, start);
                end_tool(&mut conversation, id, start);
            }
            say(&mut conversation, "Done.", start);
            conversation.end_reply(None, Some(TOKENS_OF_A_REPLY), start);
        }
        Self {
            conversation,
            layout,
            now: start + SETTLED,
        }
    }

    /// Builds a reply that has run `tools` tools in a row, the last one started a moment ago,
    /// so that each frame shows it arriving.
    #[must_use]
    pub fn with_tool_arriving(tools: usize) -> Self {
        let layout = Layout::reflowed(WINDOW, RETINA);
        let start = Instant::now();
        let mut conversation = Conversation::default();
        send(&mut conversation, "look around", start);
        for tool in 0..tools {
            let id = tool.to_string();
            start_tool(&mut conversation, &id, start + SETTLED);
            if tool + 1 < tools {
                end_tool(&mut conversation, id, start + SETTLED);
            }
        }
        Self {
            conversation,
            layout,
            now: start + SETTLED + ARRIVING,
        }
    }

    /// Composes one frame and returns how many lines its panes show.
    pub fn frame(&mut self) -> usize {
        self.conversation.prepare(&self.layout, self.now, true);
        let scene = self
            .conversation
            .scene(&self.layout, self.now, &IdTranslator);
        scene.panes.iter().map(|pane| pane.lines.len()).sum()
    }
}

impl TintBench {
    /// Composes the frame of a conversation of `exchanges` prompts whose replies each run
    /// `tools` tools, as [`ConversationBench::with_tools`] does.
    #[must_use]
    pub fn with_tools(exchanges: usize, tools: usize) -> Self {
        let mut bench = ConversationBench::with_tools(exchanges, tools);
        bench.conversation.prepare(&bench.layout, bench.now, true);
        let scene = bench
            .conversation
            .scene(&bench.layout, bench.now, &IdTranslator);
        Self { panes: scene.panes }
    }

    /// Copies the frame's lines and lays the colours of meaning on them, as each frame does,
    /// and returns how many pieces of text they hold.
    #[must_use]
    pub fn tint(&self) -> usize {
        let mut panes = self.panes.clone();
        let tints = Tints {
            tool: Some(SESSION_COLOUR),
            states: true,
            stopped: STOPPED,
        };
        tint(&mut panes, &tints);
        pieces(&panes)
    }

    /// Copies the frame's lines only, to tell the cost of the copy from that of the colours.
    #[must_use]
    pub fn copy(&self) -> usize {
        pieces(&self.panes.clone())
    }
}

fn pieces(panes: &[Pane]) -> usize {
    panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .map(|placed| placed.line.spans.len())
        .sum()
}

impl ReplyBench {
    /// Starts a reply already `length` bytes long, made of short words and line breaks.
    #[must_use]
    pub fn of_length(length: usize) -> Self {
        let now = Instant::now();
        let mut conversation = Conversation::default();
        send(&mut conversation, "write a long answer", now);
        let text = "word ".repeat(length / 5);
        say(&mut conversation, &text, now);
        Self { conversation, now }
    }

    /// Starts a reply already `length` bytes long, shown word by word as the window shows it.
    #[must_use]
    pub fn paced_of_length(length: usize) -> Self {
        let mut bench = Self::of_length(length);
        bench.conversation.pace_replies();
        bench
    }

    /// Hands the reply one more piece, as it arrives from Claude.
    pub fn hear(&mut self, piece: &str) {
        self.now += Duration::from_millis(1);
        say(&mut self.conversation, piece, self.now);
    }

    /// Hands the reply one more piece a frame later, then shows what the pace allows of it, as
    /// each frame does.
    pub fn hear_a_frame_later(&mut self, piece: &str) {
        self.now += FRAME;
        say(&mut self.conversation, piece, self.now);
        self.conversation.flow(self.now, &IdTranslator);
    }
}

pub(super) fn send(conversation: &mut Conversation, text: &str, now: Instant) {
    let insert = Intent::Edit(Edit::Insert(text.to_owned()));
    conversation.type_key(insert, now, || None, &IdTranslator);
    conversation.type_key(Intent::Send, now, || None, &IdTranslator);
}

pub(super) fn say(conversation: &mut Conversation, text: &str, now: Instant) {
    let piece = ReplyEvent::Text(text.to_owned());
    conversation.hear(piece, now, &IdTranslator);
}

pub(super) fn start_tool(conversation: &mut Conversation, id: &str, now: Instant) {
    let tool = ToolUse {
        kind: ToolKind::Read,
        subject: TOOL_SUBJECT.to_owned(),
    };
    let started = ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool,
    };
    conversation.hear(started, now, &IdTranslator);
}

pub(super) fn end_tool(conversation: &mut Conversation, id: String, now: Instant) {
    let end = ToolEnd::Done(Detail::Lines(TOOL_LINES));
    conversation.hear(ReplyEvent::ToolEnded { id, end }, now, &IdTranslator);
}
