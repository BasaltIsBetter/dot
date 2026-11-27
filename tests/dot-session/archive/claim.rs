use std::time::{Duration, Instant};

use super::*;
use crate::git::tests::ScratchFolder;

const PATIENCE: Duration = Duration::from_secs(5);

#[test]
fn the_first_dot_keeps_the_sessions_and_a_second_does_not() {
    let folder = ScratchFolder::new("claim-two");
    let first = Claim::new(Some(folder.path().join("sessions")));
    let second = Claim::new(Some(folder.path().join("sessions")));

    assert_eq!(first.ours(), Ok(()));
    assert_eq!(second.ours(), Err(ArchiveError(ANOTHER_DOT.to_owned())));
    assert_eq!(first.ours(), Ok(()));
}

#[test]
fn a_claim_let_go_can_be_taken_again() {
    let folder = ScratchFolder::new("claim-again");
    let first = Claim::new(Some(folder.path().to_path_buf()));
    assert_eq!(first.ours(), Ok(()));

    drop(first);

    let started = Instant::now();
    while Claim::new(Some(folder.path().to_path_buf()))
        .ours()
        .is_err()
    {
        assert!(started.elapsed() < PATIENCE, "the claim was never let go");
        std::thread::yield_now();
    }
}

#[test]
fn without_a_folder_nothing_is_ours() {
    assert_eq!(Claim::new(None).ours(), Err(Claim::no_folder()));
}

#[test]
fn a_folder_that_cannot_be_made_is_not_ours() {
    let folder = ScratchFolder::new("claim-file");
    let file = folder.write("taken", "a file, not a folder");

    assert!(Claim::new(Some(file.join("sessions"))).ours().is_err());
}
