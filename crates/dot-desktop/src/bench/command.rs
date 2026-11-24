use std::time::{Duration, Instant};

use dot_api::{Detail, ReplyEvent, ToolEnd, ToolKind, ToolUse};

use crate::bench::IdTranslator;
use crate::bench::conversation::{
    ARRIVING, ConversationBench, RETINA, SETTLED, TOKENS_OF_A_REPLY, WINDOW, say, send,
};
use crate::canvas::Layout;
use crate::conversation::Conversation;

const COMMANDS: [&str; 3] = [
    "cargo test -p dot-desktop",
    "cargo clippy --workspace --all-targets",
    "cargo fmt --all --check",
];
const OUTPUT: [&str; 3] = [
    "   Compiling dot-desktop v0.1.0 (crates/dot-desktop)",
    "     Running unittests src/lib.rs",
    "test canvas::layout::tests::grid_is_computed_once ... ok",
];

/// A reply that runs commands one after the other, as Claude reports them.
pub struct CommandBench {
    conversation: Conversation,
    now: Instant,
    commands: usize,
}

impl CommandBench {
    /// Starts a reply that has already run `commands` commands in a row.
    #[must_use]
    pub fn after(commands: usize) -> Self {
        let now = Instant::now();
        let mut conversation = Conversation::default();
        send(&mut conversation, "run the checks", now);
        let mut bench = Self {
            conversation,
            now,
            commands: 0,
        };
        for _ in 0..commands {
            bench.run_command();
        }
        bench
    }

    /// Runs one more command: started, then ended with its exit code.
    pub fn run_command(&mut self) {
        self.now += Duration::from_millis(1);
        self.commands += 1;
        let id = self.commands.to_string();
        start_command(&mut self.conversation, &id, self.commands, self.now);
        end_command(&mut self.conversation, id, self.now);
    }
}

impl ConversationBench {
    /// Builds a conversation of `exchanges` prompts whose replies each say a few words, run
    /// `commands` commands in a row and say a few more words, the view at the end and every
    /// animation over.
    #[must_use]
    pub fn with_commands(exchanges: usize, commands: usize) -> Self {
        let start = Instant::now();
        let mut conversation = Conversation::default();
        for exchange in 0..exchanges {
            send(&mut conversation, &format!("question {exchange}"), start);
            say(&mut conversation, "Let me check.", start);
            for command in 0..commands {
                let id = format!("{exchange}-{command}");
                start_command(&mut conversation, &id, command, start);
                end_command(&mut conversation, id, start);
            }
            say(&mut conversation, "Done.", start);
            conversation.end_reply(None, Some(TOKENS_OF_A_REPLY), start);
        }
        Self {
            conversation,
            layout: Layout::reflowed(WINDOW, RETINA),
            now: start + SETTLED,
        }
    }

    /// Builds a reply that has run `commands` commands in a row, the last one ended a moment
    /// ago, and runs one more, started then, whose curtain shows what it prints.
    #[must_use]
    pub fn with_command_running(commands: usize) -> Self {
        let start = Instant::now();
        let ended = start + SETTLED;
        let mut conversation = Conversation::default();
        send(&mut conversation, "run the checks", start);
        say(&mut conversation, "Let me check.", start);
        for command in 0..commands {
            let id = command.to_string();
            let at = if command + 1 < commands { start } else { ended };
            start_command(&mut conversation, &id, command, at);
            end_command(&mut conversation, id, at);
        }
        let running = commands.to_string();
        start_command(&mut conversation, &running, commands, ended);
        let lines = OUTPUT.iter().map(|line| (*line).to_owned()).collect();
        let output = ReplyEvent::ToolOutput { id: running, lines };
        conversation.hear(output, ended, &IdTranslator);
        Self {
            conversation,
            layout: Layout::reflowed(WINDOW, RETINA),
            now: ended + ARRIVING,
        }
    }
}

fn start_command(conversation: &mut Conversation, id: &str, order: usize, now: Instant) {
    let tool = ToolUse {
        kind: ToolKind::Run,
        subject: COMMANDS[order % COMMANDS.len()].to_owned(),
    };
    let started = ReplyEvent::ToolStarted {
        id: id.to_owned(),
        tool,
    };
    conversation.hear(started, now, &IdTranslator);
}

fn end_command(conversation: &mut Conversation, id: String, now: Instant) {
    let end = ToolEnd::Done(Detail::Exit(0));
    conversation.hear(ReplyEvent::ToolEnded { id, end }, now, &IdTranslator);
}
