use std::fs;

use dot_api::{Hue, Journal, KeptSession};

use super::*;
use crate::archive::journal::tests::{asked, heard};
use crate::git::tests::ScratchFolder;

fn kept(id: u64, name: Option<&str>) -> KeptSession {
    KeptSession {
        id: KeptId::new(id),
        name: name.map(str::to_owned),
        hue: Hue::Cyan,
        folder: Some(PathBuf::from("/Users/me/Dot")),
        model: None,
        effort: None,
    }
}

fn sessions_in(folder: &ScratchFolder) -> PathBuf {
    folder.path().join("sessions")
}

#[test]
fn keeps_nothing_without_a_folder() {
    let archive = FileArchive::new(None);

    assert_eq!(archive.restore(&[]), Shelf::default());
    assert!(archive.arrange(&Arrangement::default()).is_err());
    assert!(
        archive
            .journal(KeptId::new(1))
            .write(&asked(0, "a"))
            .is_err()
    );
}

#[test]
fn restores_nothing_at_the_first_launch() {
    let folder = ScratchFolder::new("archive-first");

    assert_eq!(
        FileArchive::new(Some(sessions_in(&folder))).restore(&[]),
        Shelf::default()
    );
}

#[test]
fn restores_the_sessions_it_kept_in_order_with_their_conversations() {
    let folder = ScratchFolder::new("archive-kept");
    let archive = FileArchive::new(Some(sessions_in(&folder)));
    let mut journal = archive.journal(KeptId::new(2));
    journal.write(&asked(0, "Salut")).unwrap();
    journal.write(&heard(10, "Bonjour")).unwrap();
    journal
        .keep_memory(&|memory: &mut dyn std::io::Write| memory.write_all(b"remembered"))
        .unwrap();
    drop(journal);
    let arrangement = Arrangement {
        sessions: vec![kept(2, Some("refonte")), kept(1, None)],
        shown: Some(KeptId::new(1)),
    };
    archive.arrange(&arrangement).unwrap();

    let shelf = archive.restore(&[]);

    assert_eq!(shelf.shown, Some(KeptId::new(1)));
    assert_eq!(
        shelf.sessions,
        [
            Restored {
                session: kept(2, Some("refonte")),
                records: vec![asked(0, "Salut"), heard(10, "Bonjour")],
                memory: Some(b"remembered".to_vec()),
            },
            Restored {
                session: kept(1, None),
                records: Vec::new(),
                memory: None,
            },
        ]
    );
}

#[test]
fn forgets_the_sessions_closed_but_not_those_of_this_launch() {
    let folder = ScratchFolder::new("archive-forget");
    let archive = FileArchive::new(Some(sessions_in(&folder)));
    for id in 1..=3 {
        archive
            .journal(KeptId::new(id))
            .write(&asked(0, "a"))
            .unwrap();
    }
    archive
        .arrange(&Arrangement {
            sessions: vec![kept(1, None)],
            shown: None,
        })
        .unwrap();
    folder.write("sessions/notes/keep.txt", "not a session");

    let shelf = archive.restore(&[KeptId::new(3)]);

    let sessions = sessions_in(&folder);
    assert_eq!(shelf.sessions.len(), 1);
    assert!(sessions.join("0000000000000001").exists());
    assert!(!sessions.join("0000000000000002").exists());
    assert!(sessions.join("0000000000000003").exists());
    assert!(sessions.join("notes/keep.txt").exists());
}

#[test]
fn forgets_nothing_when_its_index_cannot_be_read() {
    let folder = ScratchFolder::new("archive-damaged");
    let archive = FileArchive::new(Some(sessions_in(&folder)));
    archive
        .journal(KeptId::new(1))
        .write(&asked(0, "a"))
        .unwrap();
    folder.write("sessions/index.json", "{damaged");

    let shelf = archive.restore(&[]);

    assert_eq!(shelf, Shelf::default());
    assert!(sessions_in(&folder).join("0000000000000001").exists());
}

#[test]
fn a_second_dot_keeps_nothing() {
    let folder = ScratchFolder::new("archive-second");
    let first = FileArchive::new(Some(sessions_in(&folder)));
    first
        .arrange(&Arrangement {
            sessions: vec![kept(1, None)],
            shown: None,
        })
        .unwrap();

    let second = FileArchive::new(Some(sessions_in(&folder)));

    assert_eq!(second.restore(&[]), Shelf::default());
    assert!(second.arrange(&Arrangement::default()).is_err());
    assert!(
        second
            .journal(KeptId::new(1))
            .write(&heard(0, "a"))
            .is_err()
    );
    assert_eq!(first.restore(&[]).sessions.len(), 1);
}

#[test]
fn keeps_its_index_for_its_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let folder = ScratchFolder::new("archive-private");

    FileArchive::new(Some(sessions_in(&folder)))
        .arrange(&Arrangement::default())
        .unwrap();

    let mode = |path: PathBuf| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(sessions_in(&folder)), 0o700);
    assert_eq!(mode(sessions_in(&folder).join(INDEX)), 0o600);
}
