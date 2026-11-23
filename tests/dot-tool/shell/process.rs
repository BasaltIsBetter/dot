use std::time::Instant;

use super::*;

fn ran(script: &str) -> Ran {
    execute(
        script,
        None,
        None,
        (Duration::from_secs(10), &Cancellation::new()),
        &mut |_| {},
    )
    .unwrap()
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
        None,
        (Duration::from_secs(10), &Cancellation::new()),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(done.capture.heard("unused").text, "/usr\n");
}

#[test]
fn looks_for_programs_in_the_path_it_is_given() {
    let done = execute(
        "printenv PATH",
        None,
        Some(OsStr::new("/opt/dot-test/bin:/usr/bin:/bin")),
        (Duration::from_secs(10), &Cancellation::new()),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(
        done.capture.heard("unused").text,
        "/opt/dot-test/bin:/usr/bin:/bin\n"
    );
}

#[test]
fn keeps_dot_s_own_path_without_one() {
    let done = ran("printenv PATH");

    let own = std::env::var("PATH").unwrap();
    assert_eq!(done.capture.heard("unused").text, format!("{own}\n"));
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

    let done = execute(
        "sleep 30",
        None,
        None,
        (Duration::from_secs(60), &cancellation),
        &mut |_| {},
    )
    .unwrap();

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
        None,
        (Duration::from_millis(200), &Cancellation::new()),
        &mut |_| {},
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

#[test]
fn hands_out_what_a_running_command_prints_before_it_ends() {
    let mut pieces: Vec<Vec<u8>> = Vec::new();

    let done = execute(
        "echo first; sleep 0.6; echo second",
        None,
        None,
        (Duration::from_secs(10), &Cancellation::new()),
        &mut |piece| pieces.push(piece.to_vec()),
    )
    .unwrap();

    assert_eq!(done.ending, Ending::Exited(0));
    assert_eq!(
        pieces.first().map(Vec::as_slice),
        Some(b"first\n".as_slice())
    );
    assert_eq!(done.capture.heard("unused").text, "first\nsecond\n");
}

#[test]
fn hands_out_nothing_for_a_command_that_prints_nothing() {
    let mut pieces = 0;

    execute(
        "sleep 0.3",
        None,
        None,
        (Duration::from_secs(10), &Cancellation::new()),
        &mut |_| pieces += 1,
    )
    .unwrap();

    assert_eq!(pieces, 0);
}

#[test]
fn hands_out_what_arrives_a_tenth_of_a_second_at_a_time_at_most() {
    let mut pieces: u128 = 0;
    let started = Instant::now();

    execute(
        "for line in $(seq 1 30); do echo $line; sleep 0.01; done",
        None,
        None,
        (Duration::from_secs(10), &Cancellation::new()),
        &mut |_| pieces += 1,
    )
    .unwrap();

    let tenths = started.elapsed().as_millis() / 100;
    assert!(pieces >= 1);
    assert!(
        pieces <= tenths + 2,
        "{pieces} pieces in {tenths} tenths of a second"
    );
}

struct Interrupted {
    given: Vec<io::Result<Vec<u8>>>,
}

impl Read for Interrupted {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.given.is_empty() {
            return Ok(0);
        }
        let bytes = self.given.remove(0)?;
        into[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

#[test]
fn reads_on_after_an_interruption_and_stops_at_an_error() {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let output = Interrupted {
        given: vec![
            Ok(b"a".to_vec()),
            Err(io::ErrorKind::Interrupted.into()),
            Ok(b"b".to_vec()),
            Err(io::ErrorKind::BrokenPipe.into()),
            Ok(b"c".to_vec()),
        ],
    };

    let reader = listen(output, Arc::clone(&capture));
    reader.join().unwrap();

    assert_eq!(lock(&capture).take_fresh(), b"ab");
}
