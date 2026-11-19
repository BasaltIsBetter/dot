use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Stdio;

use super::*;

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", script])
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

#[test]
fn hands_over_the_pipes_of_the_program() {
    let (running, pipes) = Running::start(shell("echo hello"), &Cancellation::new()).unwrap();
    let mut printed = String::new();

    pipes.stdout.unwrap().read_to_string(&mut printed).unwrap();

    assert_eq!(printed, "hello\n");
    assert!(pipes.stdin.is_some());
    assert!(running.finish(Duration::from_secs(5)).unwrap().success());
}

#[test]
fn stops_the_whole_group_once_cancelled() {
    let cancellation = Cancellation::new();
    let (running, pipes) = Running::start(shell("sleep 30 & sleep 30"), &cancellation).unwrap();
    let started = Instant::now();

    cancellation.cancel();
    let mut printed = String::new();
    pipes.stdout.unwrap().read_to_string(&mut printed).unwrap();
    let status = running.finish(Duration::from_secs(5)).unwrap();

    assert!(!status.success());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn stops_a_program_that_outstays_its_grace() {
    let (running, _pipes) = Running::start(shell("sleep 30"), &Cancellation::new()).unwrap();
    let started = Instant::now();

    let status = running.finish(Duration::from_millis(100)).unwrap();

    assert!(!status.success());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn leaves_a_finished_program_alone_when_cancelled_late() {
    let cancellation = Cancellation::new();
    let (running, _pipes) = Running::start(shell("exit 0"), &cancellation).unwrap();
    let status = running.finish(Duration::from_secs(5)).unwrap();

    cancellation.cancel();

    assert!(status.success());
}

#[test]
fn reports_a_program_that_cannot_start() {
    let missing = Command::new("/nonexistent/claude");

    assert!(Running::start(missing, &Cancellation::new()).is_err());
}

#[test]
fn polls_twice_as_slowly_each_time_up_to_its_longest_pause() {
    assert_eq!(longer(FIRST_POLL), Duration::from_millis(2));
    assert_eq!(longer(Duration::from_millis(32)), LONGEST_POLL);
    assert_eq!(longer(LONGEST_POLL), LONGEST_POLL);
}
