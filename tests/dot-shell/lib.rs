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

#[test]
fn says_the_shell_could_not_run_or_the_output_was_lost() {
    for kind in [io::ErrorKind::NotFound, io::ErrorKind::PermissionDenied] {
        assert_eq!(failure_code(&io::Error::from(kind)), COULD_NOT_RUN);
    }
    for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::StorageFull] {
        assert_eq!(failure_code(&io::Error::from(kind)), OUTPUT_LOST);
    }
}

#[test]
fn calls_gone_once_the_parent_is_no_longer_there() {
    let looks = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&looks);
    let (sender, receiver) = std::sync::mpsc::channel();

    watch_the_parent(
        move || counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 3,
        Duration::from_millis(1),
        move || sender.send(()).unwrap(),
    );

    receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(looks.load(std::sync::atomic::Ordering::SeqCst), 4);
    assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
}

#[test]
fn never_calls_gone_while_the_parent_is_there() {
    let (sender, receiver) = std::sync::mpsc::channel::<()>();

    watch_the_parent(
        || true,
        Duration::from_millis(1),
        move || sender.send(()).unwrap(),
    );

    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
}

fn group_running(script: &str) -> std::process::Child {
    let mut shell = dot_process::command("/bin/sh");
    shell.args(["-c", script]).stdout(Stdio::piped());
    let mut child = shell.spawn().unwrap();
    let mut byte = [0];
    std::io::Read::read(child.stdout.as_mut().unwrap(), &mut byte).unwrap();
    child
}

#[test]
fn the_reaper_asks_the_group_to_end_first() {
    let mut command = group_running(
        "trap 'exit 3' TERM; echo r; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done",
    );
    let started = std::time::Instant::now();

    let mut reaper = reap(command.id()).unwrap();

    assert_eq!(command.wait().unwrap().code(), Some(3));
    assert!(started.elapsed() < Duration::from_millis(900));
    reaper.wait().unwrap();
}

#[test]
fn the_reaper_kills_what_is_left_a_second_later() {
    let mut command = group_running(
        "trap '' TERM; echo r; i=0; while [ $i -lt 600 ]; do i=$((i+1)); sleep 0.05; done",
    );
    let started = std::time::Instant::now();

    let mut reaper = reap(command.id()).unwrap();

    assert_eq!(command.wait().unwrap().signal(), Some(9));
    assert!(started.elapsed() >= Duration::from_secs(1));
    assert!(reaper.wait().unwrap().success());
    assert!(!dot_process::group_is_alive(command.id()));
}

#[test]
fn knows_its_parent_and_no_other() {
    let parent = std::os::unix::process::parent_id();

    assert!(is_the_parent(parent));
    assert!(!is_the_parent(parent + 1));
}

#[test]
fn ends_no_group_below_the_first_a_command_can_lead() {
    assert!(!may_end(0));
    assert!(!may_end(1));
    assert!(may_end(2));
    assert!(may_end(u32::MAX));
}
