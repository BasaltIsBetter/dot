use std::time::{Duration, Instant};

use dot_api::{AgentEvent, ReplyEvent, ToolKind, ToolUse};
use winit::dpi::PhysicalPosition;

use crate::bench::IdTranslator;
use crate::bench::conversation::{ConversationBench, RETINA, SETTLED, WINDOW, say, send};
use crate::canvas::Layout;
use crate::conversation::Conversation;
use crate::event::Wheel;

const KINDS: [&str; 3] = ["Explore", "tests", "web"];
const MISSION: &str = "find where the grid is computed";
const READ: &str = "crates/dot-desktop/src/canvas/layout.rs";
const TOKENS: u64 = 480;
const WRITING: Duration = Duration::from_millis(400);
const LONG_REPLY_LINES: usize = 300;
const WHEEL_LINES_UP: f32 = 40.0;
const POINTER: PhysicalPosition<f64> = PhysicalPosition::new(1_000.0, 700.0);

impl ConversationBench {
    /// Builds a reply that waits for `agents` agents at work, each reading a file and writing,
    /// the frame a moment after they last wrote: their lanes under the reply, the instrument
    /// beside the reading line.
    #[must_use]
    pub fn with_agents_working(agents: usize) -> Self {
        let start = Instant::now();
        let mut conversation = Conversation::default();
        send(&mut conversation, "look around", start);
        say(&mut conversation, "Let me send a few agents.", start);
        start_agents(&mut conversation, agents, start);
        Self {
            conversation,
            layout: Layout::reflowed(WINDOW, RETINA),
            now: start + WRITING + SETTLED,
        }
    }

    /// Builds a reply long enough that, once the view went back up the wheel, the lanes of its
    /// `agents` agents at work lie below the window.
    #[must_use]
    pub fn with_agents_out_of_sight(agents: usize) -> Self {
        let start = Instant::now();
        let layout = Layout::reflowed(WINDOW, RETINA);
        let mut conversation = Conversation::default();
        send(&mut conversation, "write a long answer", start);
        say(
            &mut conversation,
            &"a line\n".repeat(LONG_REPLY_LINES),
            start,
        );
        start_agents(&mut conversation, agents, start);
        let wheel = Wheel::Lines(WHEEL_LINES_UP);
        let scrolled = start + WRITING;
        conversation.scroll(wheel, (POINTER, &layout), scrolled, &IdTranslator);
        Self {
            conversation,
            layout,
            now: scrolled + SETTLED,
        }
    }
}

fn start_agents(conversation: &mut Conversation, agents: usize, now: Instant) {
    for agent in 0..agents {
        let id = format!("agent-{agent}");
        let delegate = ToolUse {
            kind: ToolKind::Delegate,
            subject: MISSION.to_owned(),
        };
        hear(
            conversation,
            ReplyEvent::ToolStarted {
                id: id.clone(),
                tool: delegate,
            },
            now,
        );
        let kind = KINDS[agent % KINDS.len()].to_owned();
        hear(
            conversation,
            agent_event(&id, AgentEvent::Started { kind }),
            now,
        );
        let read = ToolUse {
            kind: ToolKind::Read,
            subject: READ.to_owned(),
        };
        let action = AgentEvent::ToolStarted {
            id: format!("{id}-read"),
            tool: read,
        };
        hear(conversation, agent_event(&id, action), now);
        let wrote = AgentEvent::Wrote { tokens: TOKENS };
        hear(conversation, agent_event(&id, wrote), now + WRITING);
    }
}

fn agent_event(id: &str, event: AgentEvent) -> ReplyEvent {
    ReplyEvent::Agent {
        id: id.to_owned(),
        event,
    }
}

fn hear(conversation: &mut Conversation, event: ReplyEvent, now: Instant) {
    conversation.hear(event, now, &IdTranslator);
}
