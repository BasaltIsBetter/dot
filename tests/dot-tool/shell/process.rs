use std::time::Instant;

use super::*;

fn ran(script: &str) -> Ran {
    execute(script, None, Duration::from_secs(10), &Cancellation::new()).unwrap()
}

#[test]
fn runs_a_script_and_tells_its_exit_status() {
    let done = ran("echo hi; exit 3");

    assert_eq!(done.ending, Ending::Exited(3));
    assert_eq!(done.capture.heard("unused").text, "hi\n");
    assert!(done.lingering.is_none());
}

#[test]
fn a_killed_script_reports_the_signal_above_128() {
    let killed = ran("kill -TERM $$");

    assert_eq!(killed.ending, Ending::Exited(128 + 15));
}

#[test]
fn starts_in_the_given_folder() {
    let done = execute(
        "pwd",
        Some(Path::new("/usr")),
        Duration::from_secs(10),
        &Cancellation::new(),
    )
    .unwrap();

    assert_eq!(done.capture.heard("unused").text, "/usr\n");
}

#[test]
fn hides_the_credentials_of_anthropic_from_commands() {
    let done = ran("echo \"[${ANTHROPIC_API_KEY-unset}][${ANTHROPIC_AUTH_TOKEN-unset}]\"");

    assert_eq!(done.capture.heard("unused").text, "[unset][unset]\n");
}

#[test]
fn reads_nothing_from_the_keyboard() {
    let started = Instant::now();
    let done = ran("cat; echo after");

    assert_eq!(done.capture.heard("unused").text, "after\n");
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn stops_a_long_command_on_cancellation_within_a_second() {
    let cancellation = Cancellation::new();
    let cancelling = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        cancelling.cancel();
    });
    let started = Instant::now();

    let done = execute("sleep 30", None, Duration::from_secs(60), &cancellation).unwrap();

    canceller.join().unwrap();
    assert_eq!(done.ending, Ending::Stopped);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn stops_a_command_at_its_timeout_with_its_children() {
    let started = Instant::now();

    let done = execute(
        "echo started; sleep 30 & sleep 30",
        None,
        Duration::from_millis(200),
        &Cancellation::new(),
    )
    .unwrap();

    assert_eq!(done.ending, Ending::TimedOut);
    assert_eq!(done.capture.heard("unused").text, "started\n");
    assert!(done.lingering.is_none());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn leaves_a_background_child_running_and_stops_waiting_for_its_output() {
    let started = Instant::now();

    let done = ran("echo now; (sleep 2; echo later) &");

    assert_eq!(done.ending, Ending::Exited(0));
    assert_eq!(done.capture.heard("unused").text, "now\n");
    assert!(started.elapsed() < Duration::from_secs(1));
    let lingering = done.lingering.expect("the child still holds the output");
    assert!(!lingering.is_finished());
    let stopping = Instant::now();
    lingering.stop();
    assert!(stopping.elapsed() < Duration::from_secs(1));
}

#[test]
fn a_background_child_with_its_own_output_does_not_linger() {
    let done = ran("(sleep 2 > /dev/null 2>&1 &) ; echo quick");

    assert!(done.lingering.is_none());
}

#[test]
fn waits_a_moment_for_output_written_just_after_the_command_ends() {
    let done = ran("(sleep 0.03; echo late) & echo now");

    assert_eq!(done.capture.heard("unused").text, "now\nlate\n");
    assert!(done.lingering.is_none());
}
