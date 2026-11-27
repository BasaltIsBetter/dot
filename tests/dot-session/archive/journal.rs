use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use dot_api::{Asked, Opening, ReplyEvent};

use super::*;
use crate::archive::shelf::records_of;
use crate::git::tests::ScratchFolder;

pub(crate) fn asked(at: u64, text: &str) -> Record<'static> {
    Record {
        at: Duration::from_millis(at),
        happened: Happened::Asked(Asked {
            text: Cow::Owned(text.to_owned()),
            opening: Opening::Prompt,
            images: Vec::new(),
        }),
    }
}

pub(crate) fn heard(at: u64, text: &str) -> Record<'static> {
    Record {
        at: Duration::from_millis(at),
        happened: Happened::Heard(Cow::Owned(ReplyEvent::Text(text.to_owned()))),
    }
}

pub(crate) fn journal_in(folder: &Path) -> FileJournal {
    journal_claimed(folder, &Arc::new(Claim::new(Some(folder.to_path_buf()))))
}

fn journal_claimed(folder: &Path, claim: &Arc<Claim>) -> FileJournal {
    FileJournal::new(Some(folder.join("0000000000000001")), Arc::clone(claim))
}

fn kept_in(folder: &Path) -> PathBuf {
    folder.join("0000000000000001")
}

#[test]
fn touches_nothing_until_the_first_record() {
    let folder = ScratchFolder::new("journal-lazy");

    let mut journal = journal_in(folder.path());
    journal.flush().unwrap();
    drop(journal);

    assert!(!kept_in(folder.path()).exists());
}

#[test]
fn writes_a_header_then_one_record_per_line() {
    let folder = ScratchFolder::new("journal-lines");
    let mut journal = journal_in(folder.path());

    journal.write(&asked(0, "Salut")).unwrap();
    journal.write(&heard(1_500, "Bonjour")).unwrap();
    journal.flush().unwrap();

    assert_eq!(
        fs::read_to_string(kept_in(folder.path()).join(JOURNAL)).unwrap(),
        concat!(
            "{\"journal\":1}\n",
            "[0,{\"asked\":{\"text\":\"Salut\",\"opening\":\"prompt\"}}]\n",
            "[1500,{\"heard\":{\"text\":\"Bonjour\"}}]\n"
        )
    );
}

#[test]
fn reads_back_what_it_wrote() {
    let folder = ScratchFolder::new("journal-back");
    let mut journal = journal_in(folder.path());

    journal.write(&asked(0, "Salut")).unwrap();
    journal.write(&heard(20, "Bon")).unwrap();
    drop(journal);

    assert_eq!(
        records_of(&kept_in(folder.path())),
        [asked(0, "Salut"), heard(20, "Bon")]
    );
}

#[test]
fn goes_on_after_the_records_of_a_launch_before() {
    let folder = ScratchFolder::new("journal-again");
    let claim = Arc::new(Claim::new(Some(folder.path().to_path_buf())));
    let mut first = journal_claimed(folder.path(), &claim);
    first.write(&asked(0, "Salut")).unwrap();
    drop(first);

    let mut second = journal_claimed(folder.path(), &claim);
    second.write(&asked(9_000, "Encore")).unwrap();
    drop(second);

    assert_eq!(
        records_of(&kept_in(folder.path())),
        [asked(0, "Salut"), asked(9_000, "Encore")]
    );
}

#[test]
fn a_line_cut_short_by_a_crash_costs_only_itself() {
    let folder = ScratchFolder::new("journal-torn");
    let claim = Arc::new(Claim::new(Some(folder.path().to_path_buf())));
    let mut first = journal_claimed(folder.path(), &claim);
    first.write(&asked(0, "Salut")).unwrap();
    drop(first);
    let path = kept_in(folder.path()).join(JOURNAL);
    let mut torn = fs::read(&path).unwrap();
    torn.extend_from_slice(b"[5,{\"heard\":{\"te");
    fs::write(&path, torn).unwrap();

    let mut second = journal_claimed(folder.path(), &claim);
    second.write(&asked(9_000, "Encore")).unwrap();
    drop(second);

    assert_eq!(
        records_of(&kept_in(folder.path())),
        [asked(0, "Salut"), asked(9_000, "Encore")]
    );
}

#[test]
fn keeps_its_files_for_their_owner_only() {
    let folder = ScratchFolder::new("journal-private");
    let mut journal = journal_in(folder.path());

    journal.write(&asked(0, "Salut")).unwrap();
    journal
        .keep_memory(&|memory: &mut dyn Write| memory.write_all(b"remembered"))
        .unwrap();
    drop(journal);

    let kept = kept_in(folder.path());
    let mode = |path: PathBuf| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(kept.clone()), 0o700);
    assert_eq!(mode(kept.join(JOURNAL)), 0o600);
    assert_eq!(mode(kept.join(MEMORY)), 0o600);
}

#[test]
fn keeps_the_latest_memory_whole() {
    let folder = ScratchFolder::new("journal-memory");
    let mut journal = journal_in(folder.path());

    journal
        .keep_memory(&|memory: &mut dyn Write| memory.write_all(b"first"))
        .unwrap();
    let failed = journal.keep_memory(&|memory: &mut dyn Write| {
        memory.write_all(b"half")?;
        Err(io::Error::other("the chat failed"))
    });
    let memory = fs::read(kept_in(folder.path()).join(MEMORY)).unwrap();
    journal
        .keep_memory(&|memory: &mut dyn Write| memory.write_all(b"second"))
        .unwrap();

    assert_eq!(failed, Err(ArchiveError("the chat failed".to_owned())));
    assert_eq!(memory, b"first");
    assert_eq!(
        fs::read(kept_in(folder.path()).join(MEMORY)).unwrap(),
        b"second"
    );
}

#[test]
fn never_writes_through_a_link() {
    let folder = ScratchFolder::new("journal-link");
    let elsewhere = folder.write("elsewhere", "");
    fs::create_dir_all(kept_in(folder.path())).unwrap();
    symlink(&elsewhere, kept_in(folder.path()).join(JOURNAL)).unwrap();

    let written = journal_in(folder.path()).write(&asked(0, "Salut"));

    assert_eq!(written, Err(ArchiveError(NOT_A_FILE.to_owned())));
    assert_eq!(fs::read(&elsewhere).unwrap(), b"");
}

#[test]
fn keeps_nothing_without_a_folder_or_the_claim() {
    let folder = ScratchFolder::new("journal-unclaimed");
    let claimed = Claim::new(Some(folder.path().to_path_buf()));
    claimed.ours().unwrap();
    let mut without_folder = FileJournal::new(None, Arc::new(Claim::new(None)));
    let mut unclaimed = journal_in(folder.path());

    assert_eq!(
        without_folder.write(&asked(0, "a")),
        Err(Claim::no_folder())
    );
    assert!(unclaimed.write(&asked(0, "a")).is_err());
    assert!(
        unclaimed
            .keep_memory(&|memory: &mut dyn Write| memory.write_all(b"a"))
            .is_err()
    );
    assert!(!kept_in(folder.path()).exists());
}

#[test]
fn writes_each_time_on_the_millisecond_without_overflowing() {
    let mut line = Vec::new();
    let far = Record {
        at: Duration::MAX,
        happened: Happened::StartedOver,
    };

    line_of(&far, &mut line).unwrap();

    assert_eq!(
        line,
        format!("[{},\"started-over\"]\n", u64::MAX).into_bytes()
    );
    assert_eq!(
        record_of(&line[..line.len() - 1]).map(|record| record.at),
        Some(Duration::from_millis(u64::MAX))
    );
}

#[test]
fn recognises_only_its_own_header() {
    assert!(is_header(HEADER));
    assert!(!is_header(br#"{"journal":2}"#));
    assert!(!is_header(b"[0,\"started-over\"]"));
    assert!(!is_header(b""));
}
