use std::borrow::Cow;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use dot_api::{
    ArchiveError, Asked, Detail, Ending, Happened, Journal, Opening, Record, ReplyEvent, ToolEnd,
    ToolKind, ToolUse,
};

use crate::bench::IdTranslator;
use crate::conversation::Conversation;
use crate::reply::{Clock, Recorder};

const PIECES_PER_REPLY: usize = 20;
const PIECE: &str = "a few more words ";
const BETWEEN_PIECES: u64 = 120;

/// What the thread of a reply keeps of each piece that streams in, before handing it on.
pub struct RecorderBench {
    recorder: Recorder<CountingJournal>,
    now: Instant,
    piece: ReplyEvent,
    tool: ReplyEvent,
}

/// A conversation kept in a journal, shown again as the window does when Dot starts.
pub struct ReplayBench {
    records: Vec<Record<'static>>,
}

struct CountingJournal {
    written: usize,
}

impl RecorderBench {
    /// Prepares a recorder whose journal only counts what it is handed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            recorder: Recorder::new(CountingJournal { written: 0 }, Clock::default()),
            now: Instant::now(),
            piece: ReplyEvent::Text(PIECE.to_owned()),
            tool: ReplyEvent::ToolStarted {
                id: "toolu_01".to_owned(),
                tool: ToolUse {
                    kind: ToolKind::Run,
                    subject: "cargo test --workspace".to_owned(),
                },
            },
        }
    }

    /// Keeps a piece of text a millisecond after the one before, merged with it.
    pub fn piece(&mut self) {
        self.now += Duration::from_millis(1);
        self.recorder.heard(&self.piece, self.now);
    }

    /// Keeps a tool that starts, after the text before it.
    pub fn tool(&mut self) {
        self.now += Duration::from_millis(1);
        self.recorder.heard(&self.tool, self.now);
    }
}

impl Default for RecorderBench {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayBench {
    /// Prepares the journal of `exchanges` exchanges, each with its prompt, its text in
    /// merged pieces, a command and its end.
    #[must_use]
    pub fn of(exchanges: usize) -> Self {
        let mut records = Vec::new();
        let mut at = 0;
        for exchange in 0..exchanges {
            let mut add = |step: u64, happened: Happened<'static>| {
                at += step;
                records.push(Record {
                    at: Duration::from_millis(at),
                    happened,
                });
            };
            add(
                5_000,
                Happened::Asked(Asked {
                    text: Cow::Owned(format!("prompt number {exchange}, a sentence long")),
                    opening: Opening::Prompt,
                    images: Vec::new(),
                }),
            );
            for _ in 0..PIECES_PER_REPLY {
                add(BETWEEN_PIECES, heard(ReplyEvent::Text(PIECE.repeat(4))));
            }
            add(10, heard(started()));
            add(800, heard(ended()));
            add(10, Happened::Ended(Ending::default()));
        }
        Self { records }
    }

    /// Shows the conversation again and returns whether it holds exchanges.
    #[must_use]
    pub fn replay(&self) -> bool {
        let mut conversation = Conversation::default();
        conversation.replay(self.records.clone(), Instant::now(), &IdTranslator);
        conversation.has_exchanges()
    }

    /// Copies the records only, which [`ReplayBench::replay`] does first.
    #[must_use]
    pub fn copy(&self) -> usize {
        self.records.clone().len()
    }
}

impl Journal for CountingJournal {
    fn write(&mut self, record: &Record<'_>) -> Result<(), ArchiveError> {
        self.written += usize::from(matches!(record.happened, Happened::Heard(_)));
        Ok(())
    }

    fn flush(&mut self) -> Result<(), ArchiveError> {
        Ok(())
    }

    fn keep_memory(
        &mut self,
        _chat: &dyn Fn(&mut dyn Write) -> io::Result<()>,
    ) -> Result<(), ArchiveError> {
        Ok(())
    }
}

fn heard(event: ReplyEvent) -> Happened<'static> {
    Happened::Heard(Cow::Owned(event))
}

fn started() -> ReplyEvent {
    ReplyEvent::ToolStarted {
        id: "toolu_01".to_owned(),
        tool: ToolUse {
            kind: ToolKind::Run,
            subject: "cargo test".to_owned(),
        },
    }
}

fn ended() -> ReplyEvent {
    ReplyEvent::ToolEnded {
        id: "toolu_01".to_owned(),
        end: ToolEnd::Done(Detail::Exit(0)),
    }
}
