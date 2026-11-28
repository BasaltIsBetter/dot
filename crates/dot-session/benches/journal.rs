//! Benches of what keeps the sessions on the disk: `cargo bench -p dot-session --bench journal`.

use std::borrow::Cow;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use dot_api::{
    Archive, Arrangement, Asked, Ending, Happened, Hue, Journal, KeptId, KeptSession, Opening,
    Record, ReplyEvent, ToolKind, ToolUse,
};
use dot_session::FileArchive;

const EXCHANGES: usize = 1_000;
const PIECES_PER_REPLY: usize = 20;
const PIECE: &str = "a few more words a few more words a few more words ";

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("could not remove {}: {error}", self.0.display());
        }
    }
}

fn scratch(name: &str) -> Scratch {
    let folder = std::env::temp_dir().join(format!("dot-bench-{}-{name}", std::process::id()));
    if folder.exists() {
        fs::remove_dir_all(&folder).expect("an earlier bench folder can be removed");
    }
    Scratch(folder)
}

fn record(at: u64, happened: Happened<'_>) -> Record<'_> {
    Record {
        at: Duration::from_millis(at),
        happened,
    }
}

fn writing_records(criterion: &mut Criterion) {
    let folder = scratch("write");
    let archive = FileArchive::new(Some(folder.0.clone()));
    let mut journal = archive.journal(KeptId::new(1));
    let piece = ReplyEvent::Text(PIECE.to_owned());
    let tool = ReplyEvent::ToolStarted {
        id: "toolu_01".to_owned(),
        tool: ToolUse {
            kind: ToolKind::Run,
            subject: "cargo test --workspace".to_owned(),
        },
    };
    criterion.bench_function("journal/piece_written", |bench| {
        bench.iter(|| {
            journal
                .write(&record(1, Happened::Heard(Cow::Borrowed(&piece))))
                .expect("the scratch folder takes the journal");
        });
    });
    criterion.bench_function("journal/tool_written", |bench| {
        bench.iter(|| {
            journal
                .write(&record(1, Happened::Heard(Cow::Borrowed(&tool))))
                .expect("the scratch folder takes the journal");
        });
    });
    journal.flush().expect("the journal is flushed");
}

fn restoring_a_long_session(criterion: &mut Criterion) {
    let folder = scratch("restore");
    let archive = FileArchive::new(Some(folder.0.clone()));
    let mut journal = archive.journal(KeptId::new(1));
    let piece = ReplyEvent::Text(PIECE.to_owned());
    let mut at = 0;
    for exchange in 0..EXCHANGES {
        let asked = Happened::Asked(Asked {
            text: Cow::Owned(format!("prompt number {exchange}")),
            opening: Opening::Prompt,
            images: Vec::new(),
        });
        journal.write(&record(at, asked)).expect("written");
        for _ in 0..PIECES_PER_REPLY {
            at += 100;
            let heard = Happened::Heard(Cow::Borrowed(&piece));
            journal.write(&record(at, heard)).expect("written");
        }
        let ended = Happened::Ended(Ending::default());
        journal.write(&record(at, ended)).expect("written");
    }
    journal.flush().expect("flushed");
    drop(journal);
    archive
        .arrange(&Arrangement {
            sessions: vec![KeptSession {
                id: KeptId::new(1),
                name: None,
                hue: Hue::Violet,
                folder: None,
                model: None,
                effort: None,
            }],
            shown: None,
        })
        .expect("arranged");
    let size = fs::metadata(folder.0.join("0000000000000001/journal"))
        .expect("the journal is written")
        .len();
    eprintln!("journal of {EXCHANGES} exchanges: {size} bytes");

    criterion.bench_function("journal/restore_1000_exchanges", |bench| {
        bench.iter(|| black_box(archive.restore(&[]).sessions[0].records.len()));
    });
}

criterion_group!(journal, writing_records, restoring_a_long_session);
criterion_main!(journal);
