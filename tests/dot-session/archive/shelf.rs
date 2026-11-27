use std::borrow::Cow;
use std::fs;
use std::sync::Arc;

use dot_api::{Cost, Detail, Ending, Journal, Reply, ReplyEvent, ToolEnd};

use super::*;
use crate::archive::claim::Claim;
use crate::archive::journal::FileJournal;
use crate::archive::journal::tests::{asked, heard};
use crate::git::tests::ScratchFolder;

fn ended(at: u64, micro_dollars: u64) -> Record<'static> {
    Record {
        at: Duration::from_millis(at),
        happened: Happened::Ended(Ending {
            spent: Reply {
                cost: Cost::from_micro_dollars(micro_dollars),
                ..Reply::default()
            },
            failure: None,
        }),
    }
}

fn changed(at: u64, added: u64, removed: u64) -> Record<'static> {
    Record {
        at: Duration::from_millis(at),
        happened: Happened::Heard(Cow::Owned(ReplyEvent::ToolEnded {
            id: "t".to_owned(),
            end: ToolEnd::Done(Detail::Changes { added, removed }),
        })),
    }
}

fn write_all(folder: &Path, records: &[Record<'_>]) {
    let mut journal = FileJournal::new(
        Some(folder.to_path_buf()),
        Arc::new(Claim::new(Some(folder.to_path_buf()))),
    );
    for record in records {
        journal.write(record).unwrap();
    }
}

fn size_of(record: &Record<'_>) -> usize {
    let mut line = Vec::new();
    line_of(record, &mut line).unwrap();
    line.len() - 1
}

#[test]
fn reads_nothing_without_a_journal_or_with_another_header() {
    let folder = ScratchFolder::new("shelf-none");

    assert_eq!(records_of(folder.path()), []);
    folder.write(JOURNAL, "{\"journal\":2}\n[0,\"started-over\"]\n");
    assert_eq!(records_of(folder.path()), []);
    folder.write(JOURNAL, "[0,\"started-over\"]\n");
    assert_eq!(records_of(folder.path()), []);
}

#[test]
fn a_damaged_record_costs_only_itself() {
    let folder = ScratchFolder::new("shelf-damaged");
    folder.write(
        JOURNAL,
        concat!(
            "{\"journal\":1}\n",
            "[0,{\"asked\":{\"text\":\"a\",\"opening\":\"prompt\"}}]\n",
            "[1,{\"sung\":\"la\"}]\n",
            "not json\n",
            "\n",
            "[2,{\"heard\":{\"text\":\"b\"}}]\n"
        ),
    );

    assert_eq!(records_of(folder.path()), [asked(0, "a"), heard(2, "b")]);
}

#[test]
fn skips_a_record_longer_than_it_reads() {
    let folder = ScratchFolder::new("shelf-long");
    let long = "x".repeat(LONGEST_RECORD);
    write_all(
        folder.path(),
        &[asked(0, "a"), heard(1, &long), heard(2, "b")],
    );

    assert_eq!(records_of(folder.path()), [asked(0, "a"), heard(2, "b")]);
}

#[test]
fn never_goes_back_in_time() {
    let folder = ScratchFolder::new("shelf-time");
    write_all(
        folder.path(),
        &[asked(50, "a"), heard(10, "b"), heard(60, "c")],
    );

    assert_eq!(
        records_of(folder.path()),
        [asked(50, "a"), heard(50, "b"), heard(60, "c")]
    );
}

#[test]
fn keeps_every_exchange_of_a_journal_within_its_size() {
    let folder = ScratchFolder::new("shelf-within");
    let records = [asked(0, "a"), ended(1, 5), asked(2, "b"), ended(3, 7)];
    let size: usize = records.iter().map(size_of).sum();
    write_all(folder.path(), &records);

    assert_eq!(records_within(folder.path(), size), records);
}

#[test]
fn carries_the_oldest_exchanges_of_a_journal_grown_too_large() {
    let folder = ScratchFolder::new("shelf-carried");
    let records = [
        asked(0, "a"),
        changed(1, 10, 2),
        ended(2, 5),
        asked(3, "b"),
        ended(4, 7),
        asked(5, "c"),
        ended(6, 11),
    ];
    let latest: usize = records[3..].iter().map(size_of).sum();
    write_all(folder.path(), &records);

    let read = records_within(folder.path(), latest);

    let carried = Record {
        at: Duration::from_millis(3),
        happened: Happened::Carried(Carried {
            cost: Cost::from_micro_dollars(5),
            lines_added: 10,
            lines_removed: 2,
        }),
    };
    let mut expected = vec![carried];
    expected.extend_from_slice(&records[3..]);
    assert_eq!(read, expected);
    assert_eq!(records_of(folder.path()), expected);
}

#[test]
fn keeps_the_latest_exchange_however_large() {
    let folder = ScratchFolder::new("shelf-one");
    let records = [
        asked(0, "a"),
        heard(1, "bbbbbbbbbbbbbbbbbbbbbbbbb"),
        ended(2, 1),
    ];
    write_all(folder.path(), &records);

    assert_eq!(records_within(folder.path(), 1), records);
}

#[test]
fn reads_a_memory_within_its_size_only() {
    let folder = ScratchFolder::new("shelf-memory");

    assert_eq!(memory_within(folder.path(), 4), None);
    folder.write(MEMORY, "abcd");
    assert_eq!(memory_within(folder.path(), 4), Some(b"abcd".to_vec()));
    assert_eq!(memory_within(folder.path(), 3), None);
    assert_eq!(memory_of(folder.path()), Some(b"abcd".to_vec()));
    fs::remove_file(folder.path().join(MEMORY)).unwrap();
    fs::create_dir(folder.path().join(MEMORY)).unwrap();
    assert_eq!(memory_of(folder.path()), None);
}

#[test]
fn carries_again_as_the_journal_goes_on() {
    let folder = ScratchFolder::new("shelf-again");
    let records = [
        asked(0, "a"),
        ended(1, 1),
        asked(2, "b"),
        ended(3, 2),
        asked(4, "c"),
    ];
    let one_exchange = size_of(&records[0]) + size_of(&records[1]);
    write_all(folder.path(), &records);

    let read = records_within(folder.path(), one_exchange);

    assert_eq!(read[1..], records[4..]);
    assert_eq!(
        read[0].happened,
        Happened::Carried(Carried {
            cost: Cost::from_micro_dollars(3),
            ..Carried::default()
        })
    );
}

#[test]
fn keeps_records_of_seven_mebibytes_and_journals_of_twenty() {
    let folder = ScratchFolder::new("shelf-large");
    let large = "x".repeat(7 * 1_024 * 1_024);
    let records = [
        asked(0, "a"),
        heard(1, &large),
        asked(2, "b"),
        heard(3, &large),
        asked(4, "c"),
        heard(5, &large),
    ];
    write_all(folder.path(), &records);

    assert_eq!(records_of(folder.path()), records);
}

#[test]
fn reads_a_memory_of_forty_mebibytes_but_not_more() {
    let folder = ScratchFolder::new("shelf-large-memory");
    let largest = vec![b'm'; 40 * 1_024 * 1_024];
    fs::write(folder.path().join(MEMORY), &largest).unwrap();

    assert_eq!(
        memory_of(folder.path()).map(|memory| memory.len()),
        Some(largest.len())
    );
    fs::write(folder.path().join(MEMORY), [largest, vec![b'm']].concat()).unwrap();
    assert_eq!(memory_of(folder.path()), None);
}
