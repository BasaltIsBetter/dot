use dot_api::{Archive, Cost, KeptId, ReplyFailure, Text, ToolEnd, ToolKind, ToolUse};

use super::*;
use crate::keeping::tests::{MemoryArchive, MemoryJournal};
use crate::reply::tests::{ScriptedChat, asked};

fn recorder() -> (Recorder<MemoryJournal>, MemoryArchive) {
    recorder_resuming_at(Duration::ZERO)
}

fn recorder_resuming_at(carried: Duration) -> (Recorder<MemoryJournal>, MemoryArchive) {
    let archive = MemoryArchive::default();
    let journal = archive.journal(KeptId::new(1));
    (Recorder::new(journal, Clock::resuming_at(carried)), archive)
}

fn written(archive: &MemoryArchive) -> Vec<Record<'static>> {
    archive
        .shelved()
        .journals
        .get(&KeptId::new(1))
        .cloned()
        .unwrap_or_default()
}

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn record(milliseconds: u64, happened: Happened<'static>) -> Record<'static> {
    Record {
        at: Duration::from_millis(milliseconds),
        happened,
    }
}

fn text(piece: &str) -> Happened<'static> {
    Happened::Heard(Cow::Owned(ReplyEvent::Text(piece.to_owned())))
}

fn thinking(piece: &str) -> Happened<'static> {
    Happened::Heard(Cow::Owned(ReplyEvent::Thinking(piece.to_owned())))
}

fn ended_well() -> Happened<'static> {
    Happened::Ended(Ending::default())
}

#[test]
fn keeps_what_was_asked_and_how() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();
    let opened = Opened {
        opening: Opening::Note,
        images: vec![ImageLabel {
            name: "shot.png".to_owned(),
            bytes: 9,
        }],
    };

    recorder.asked(&asked("Salut"), opened, start);

    assert_eq!(
        written(&archive),
        [record(
            0,
            Happened::Asked(Asked {
                text: Cow::Owned("Salut".to_owned()),
                opening: Opening::Note,
                images: vec![ImageLabel {
                    name: "shot.png".to_owned(),
                    bytes: 9,
                }],
            })
        )]
    );
}

#[test]
fn merges_the_pieces_that_arrive_within_a_tenth_of_a_second() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();

    recorder.heard(&ReplyEvent::Text("Bon".to_owned()), start);
    recorder.heard(&ReplyEvent::Text("jour".to_owned()), after(start, 99));
    recorder.heard(&ReplyEvent::Text(" à".to_owned()), after(start, 100));
    recorder.heard(&ReplyEvent::Thinking("hmm".to_owned()), after(start, 150));
    recorder.heard(&ReplyEvent::Text(" toi".to_owned()), after(start, 160));
    recorder.ended(&Ok(Reply::default()), Reply::default(), after(start, 170));

    assert_eq!(
        written(&archive),
        [
            record(0, text("Bonjour")),
            record(100, text(" à")),
            record(150, thinking("hmm")),
            record(160, text(" toi")),
            record(170, ended_well()),
        ]
    );
}

#[test]
fn keeps_every_other_event_in_its_place_but_the_live_output() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();
    let started = ReplyEvent::ToolStarted {
        id: "t1".to_owned(),
        tool: ToolUse {
            kind: ToolKind::Run,
            subject: "ls".to_owned(),
        },
    };
    let ended = ReplyEvent::ToolEnded {
        id: "t1".to_owned(),
        end: ToolEnd::Failed,
    };

    recorder.heard(&ReplyEvent::Text("Je liste".to_owned()), start);
    recorder.heard(&started, after(start, 10));
    recorder.heard(
        &ReplyEvent::ToolOutput {
            id: "t1".to_owned(),
            lines: vec!["a".to_owned()],
        },
        after(start, 20),
    );
    recorder.heard(&ended, after(start, 30));

    assert_eq!(
        written(&archive),
        [
            record(0, text("Je liste")),
            record(10, Happened::Heard(Cow::Owned(started))),
            record(30, Happened::Heard(Cow::Owned(ended))),
        ]
    );
}

#[test]
fn keeps_how_a_reply_ended_and_what_it_spent() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();
    let spent = Reply {
        cost: Cost::from_micro_dollars(7),
        ..Reply::default()
    };
    let problem = Problem {
        text: ReplyFailure::Overloaded.text(),
        cause: Some("HTTP 529".to_owned()),
    };

    recorder.heard(&ReplyEvent::Text("Bon".to_owned()), start);
    recorder.ended(&Err(problem), spent, after(start, 40));

    assert_eq!(
        written(&archive),
        [
            record(0, text("Bon")),
            record(
                40,
                Happened::Ended(Ending {
                    spent,
                    failure: Some(Failure {
                        text: Text::ReplyOverloaded,
                        cause: Some("HTTP 529".to_owned()),
                    }),
                })
            ),
        ]
    );
}

#[test]
fn keeps_a_fresh_start_after_what_was_waiting() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();

    recorder.heard(&ReplyEvent::Text("Bon".to_owned()), start);
    recorder.started_over(after(start, 5));

    assert_eq!(
        written(&archive),
        [record(0, text("Bon")), record(5, Happened::StartedOver)]
    );
}

#[test]
fn goes_on_from_where_the_session_stopped() {
    let (mut recorder, archive) = recorder_resuming_at(Duration::from_secs(60));
    let start = Instant::now();

    recorder.started_over(after(start, 250));
    recorder.started_over(after(start, 1_250));

    assert_eq!(
        written(&archive),
        [
            record(60_000, Happened::StartedOver),
            record(61_000, Happened::StartedOver),
        ]
    );
}

#[test]
fn never_goes_back_in_time() {
    let mut clock = Clock::resuming_at(Duration::from_secs(1));
    let start = Instant::now();

    assert_eq!(clock.at(after(start, 500)), Duration::from_secs(1));
    assert_eq!(clock.at(start), Duration::from_secs(1));
    assert_eq!(clock.at(after(start, 700)), Duration::from_millis(1_200));
}

#[test]
fn keeps_what_the_chat_remembers() {
    let (mut recorder, archive) = recorder();
    let chat = ScriptedChat {
        remembers: b"remembered".to_vec(),
        ..ScriptedChat::default()
    };

    recorder.remember(&chat);

    assert_eq!(
        archive.shelved().memories.get(&KeptId::new(1)),
        Some(&b"remembered".to_vec())
    );
}

#[test]
fn a_journal_that_cannot_be_written_is_said_once_until_it_works_again() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();
    archive.shelved().refuses = true;

    recorder.started_over(start);
    assert!(recorder.failing);
    recorder.started_over(start);
    assert!(recorder.failing);
    archive.shelved().refuses = false;
    recorder.started_over(start);

    assert!(!recorder.failing);
    assert_eq!(written(&archive), [record(0, Happened::StartedOver)]);
}

#[test]
fn a_prompt_sent_without_saying_how_is_a_prompt() {
    assert_eq!(
        Opened::default(),
        Opened {
            opening: Opening::Prompt,
            images: Vec::new(),
        }
    );
}

#[test]
fn writes_to_the_disk_at_the_end_of_each_reply_and_each_fresh_start() {
    let (mut recorder, archive) = recorder();
    let start = Instant::now();

    recorder.heard(&ReplyEvent::Text("Bon".to_owned()), start);
    assert_eq!(archive.shelved().flushes, 0);
    recorder.ended(&Ok(Reply::default()), Reply::default(), start);
    assert_eq!(archive.shelved().flushes, 1);
    recorder.started_over(start);
    assert_eq!(archive.shelved().flushes, 2);
}

struct CountingWarnings(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl tracing::Subscriber for CountingWarnings {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[test]
fn says_once_that_the_journal_cannot_be_written_until_it_can_again() {
    let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let subscriber = CountingWarnings(std::sync::Arc::clone(&warnings));
    let (mut recorder, archive) = recorder();
    let start = Instant::now();

    tracing::subscriber::with_default(subscriber, || {
        archive.shelved().refuses = true;
        recorder.started_over(start);
        recorder.started_over(start);
        archive.shelved().refuses = false;
        recorder.started_over(start);
        archive.shelved().refuses = true;
        recorder.started_over(start);
    });

    assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 2);
}
