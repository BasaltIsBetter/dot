use super::*;

#[test]
fn two_handles_on_one_file_share_it_and_two_files_do_not() {
    let folder = std::env::temp_dir().join(format!("dot-shell-share-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let first = File::create(folder.join("first")).unwrap();
    let again = first.try_clone().unwrap();
    let reopened = File::open(folder.join("first")).unwrap();
    let second = File::create(folder.join("second")).unwrap();

    assert!(share_a_file(&first, &again));
    assert!(share_a_file(&first, &reopened));
    assert!(!share_a_file(&first, &second));
    std::fs::remove_dir_all(&folder).unwrap();
}

fn ended(script: &str) -> ExitStatus {
    std::process::Command::new("/bin/sh")
        .args(["-c", script])
        .status()
        .unwrap()
}

#[test]
fn reports_the_exit_code_of_the_command_or_the_signal_that_ended_it() {
    assert_eq!(exit_code(ended("exit 0")), 0);
    assert_eq!(exit_code(ended("exit 3")), 3);
    assert_eq!(exit_code(ended("exit 255")), 255);
    assert_eq!(exit_code(ended("kill -TERM $$")), 143);
    assert_eq!(exit_code(ended("kill -KILL $$")), 137);
}

#[test]
fn holds_an_output_two_seconds_and_one_mebibyte_at_most() {
    assert_eq!(PACE.hold, Duration::from_millis(2_000));
    assert_eq!(PACE.pause, Duration::from_millis(1_000));
    assert_eq!(PACE.wait, Duration::from_secs(30));
    assert_eq!(PACE.held, 1_048_576);
    assert_eq!(COULD_NOT_RUN, 127);
}

#[test]
fn keeps_passing_on_what_is_written_after_the_command_until_nothing_writes() {
    use std::io::{Read, Write};

    let (output, mut still_writing) = io::pipe().unwrap();
    let (read_back, to) = io::pipe().unwrap();

    let mut copier = leave_passing_on(output, Stdio::from(to)).unwrap();
    still_writing.write_all(b"late\n").unwrap();
    still_writing.write_all(&[0xff, b'\n']).unwrap();
    drop(still_writing);
    let mut passed = Vec::new();
    read_back.take(64).read_to_end(&mut passed).unwrap();

    assert_eq!(passed, b"late\n\xff\n");
    assert!(copier.wait().unwrap().success());
}
