use std::time::{Duration, Instant};

use dot_api::{Detail, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::dpi::PhysicalSize;

use crate::bench::IdTranslator;
use crate::canvas::Layout;
use crate::conversation::{Conversation, Edit, Intent};

pub(super) const WINDOW: PhysicalSize<u32> = PhysicalSize::new(2_000, 1_400);
pub(super) const RETINA: f64 = 2.0;
const SETTLED: Duration = Duration::from_secs(5);
const ARRIVING: Duration = Duration::from_millis(100);
const TOOL_SUBJECT: &str = "crates/dot-desktop/src/conversation/transcript/history/tool.rs";
const TOOL_LINES: u64 = 227;
const TOKENS_OF_A_REPLY: u64 = 12;

/// A conversation with a long history, composed into frames like the window composes them.
pub struct ConversationBench {
    conversation: Conversation,
    layout: Layout,
    now: Instant,
}

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

    /// Hands the reply one more piece, as it arrives from Claude.
    pub fn hear(&mut self, piece: &str) {
        self.now += Duration::from_millis(1);
        say(&mut self.conversation, piece, self.now);
    }
}

pub(super) fn send(conversation: &mut Conversation, text: &str, now: Instant) {
    let insert = Intent::Edit(Edit::Insert(text.to_owned()));
    conversation.type_key(insert, now, || None, &IdTranslator);
    conversation.type_key(Intent::Send, now, || None, &IdTranslator);
}

fn say(conversation: &mut Conversation, text: &str, now: Instant) {
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
