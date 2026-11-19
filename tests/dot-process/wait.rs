use std::time::Duration;

use super::*;
use crate::launch::{command, stop};

#[test]
fn sees_a_quick_program_end_at_once() {
    let mut child = command("/usr/bin/true").spawn().unwrap();
    let started = Instant::now();

    let waited = wait(&mut child, None, &Cancellation::new()).unwrap();

    assert!(matches!(waited, Waited::Exited(status) if status.success()));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn gives_up_at_the_deadline_and_leaves_the_program_running() {
    let mut child = command("/bin/sleep").arg("30").spawn().unwrap();
    let started = Instant::now();

    let waited = wait(
        &mut child,
        Some(started + Duration::from_millis(100)),
        &Cancellation::new(),
    )
    .unwrap();

    assert_eq!(waited, Waited::TimedOut);
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(child.try_wait().unwrap().is_none());
    stop(&mut child).unwrap();
}

#[test]
fn ends_as_soon_as_the_task_is_cancelled() {
    let mut child = command("/bin/sleep").arg("30").spawn().unwrap();
    let cancellation = Cancellation::new();
    let canceller = cancellation.clone();
    let started = Instant::now();

    let waited = std::thread::scope(|scope| {
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            canceller.cancel();
        });
        wait(&mut child, None, &cancellation).unwrap()
    });

    assert_eq!(waited, Waited::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(2));
    stop(&mut child).unwrap();
}

#[test]
fn a_program_that_ended_counts_before_a_cancellation() {
    let mut child = command("/usr/bin/true").spawn().unwrap();
    child.wait().unwrap();
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let waited = wait(&mut child, Some(Instant::now()), &cancellation).unwrap();

    assert!(matches!(waited, Waited::Exited(_)));
}

#[test]
fn waits_twice_as_long_each_time_up_to_fifty_milliseconds() {
    assert_eq!(longer(FIRST_POLL), Duration::from_millis(2));
    assert_eq!(longer(Duration::from_millis(16)), Duration::from_millis(32));
    assert_eq!(longer(Duration::from_millis(32)), Duration::from_millis(50));
    assert_eq!(longer(LONGEST_POLL), LONGEST_POLL);
}
