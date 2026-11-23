use std::io::Read;
use std::process::Stdio;

use super::*;

#[derive(Default)]
pub(super) struct Recording {
    pub(super) wholes: Vec<String>,
    pub(super) pieces: Vec<String>,
    pub(super) heard: Vec<String>,
    pub(super) rests: usize,
    pub(super) replaces: bool,
}

impl Relay for Recording {
    fn whole(&mut self, output: &[u8]) -> Option<Vec<u8>> {
        self.wholes
            .push(String::from_utf8_lossy(output).into_owned());
        self.replaces.then(|| b"<whole>".to_vec())
    }

    fn piece(&mut self, piece: &[u8]) -> Vec<u8> {
        self.pieces
            .push(String::from_utf8_lossy(piece).into_owned());
        piece.to_ascii_uppercase()
    }

    fn rest(&mut self) -> Vec<u8> {
        self.rests += 1;
        if self.rests == 1 {
            b"<rest>".to_vec()
        } else {
            Vec::new()
        }
    }

    fn heard(&mut self, piece: &[u8]) {
        self.heard.push(String::from_utf8_lossy(piece).into_owned());
    }
}

pub(super) struct Closed;

impl Write for Closed {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", script])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

const fn pace(hold: u64, pause: u64, most: usize) -> Pace {
    Pace {
        hold: Duration::from_millis(hold),
        pause: Duration::from_millis(pause),
        wait: Duration::from_secs(60),
        held: most,
    }
}

fn relayed(
    script: &str,
    errors: Errors,
    pace: Pace,
    recording: &mut Recording,
) -> (String, Relayed) {
    let mut written = Vec::new();
    let ended = relay(shell(script), errors, pace, recording, &mut written).unwrap();
    (String::from_utf8(written).unwrap(), ended)
}

#[test]
fn shows_the_whole_output_of_a_quick_program_to_the_relay_once() {
    let mut recording = Recording {
        replaces: true,
        ..Recording::default()
    };

    let (written, ended) = relayed(
        "echo one; echo two; exit 3",
        Errors::Joined,
        pace(5_000, 1_000, 1_024),
        &mut recording,
    );

    assert_eq!(recording.wholes, ["one\ntwo\n"]);
    assert_eq!(recording.pieces, [""; 0]);
    assert_eq!(recording.rests, 0);
    assert_eq!(written, "<whole>");
    assert_eq!(ended.status.code(), Some(3));
    assert!(ended.left_open.is_none());
}

#[test]
fn writes_the_output_as_it_is_when_the_relay_has_nothing_shorter() {
    let mut recording = Recording::default();

    let (written, ended) = relayed(
        "printf 'a\\nb'",
        Errors::Joined,
        pace(5_000, 1_000, 1_024),
        &mut recording,
    );

    assert_eq!(written, "a\nb");
    assert!(ended.status.success());
}

#[test]
fn passes_on_the_output_of_a_program_that_outlasts_the_hold() {
    let mut recording = Recording::default();
    let started = Instant::now();

    let (written, ended) = relayed(
        "echo early; sleep 0.6; echo late",
        Errors::Joined,
        pace(150, 5_000, 1_024),
        &mut recording,
    );

    assert_eq!(recording.wholes, [""; 0]);
    assert_eq!(recording.pieces, ["early\n", "late\n"]);
    assert_eq!(written, "EARLY\nLATE\n<rest>");
    assert!(ended.status.success());
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn passes_on_an_output_larger_than_it_may_hold_without_waiting() {
    let mut recording = Recording::default();

    let (written, _) = relayed(
        "printf 'abcdefgh'; printf 'ij'",
        Errors::Joined,
        pace(60_000, 5_000, 4),
        &mut recording,
    );

    assert_eq!(recording.wholes, [""; 0]);
    assert_eq!(recording.pieces.concat(), "abcdefghij");
    assert_eq!(written, "ABCDEFGHIJ<rest>");
}

#[test]
fn an_output_of_exactly_what_it_may_hold_is_still_whole() {
    let mut recording = Recording::default();

    relayed(
        "printf 'abcd'",
        Errors::Joined,
        pace(60_000, 5_000, 4),
        &mut recording,
    );

    assert_eq!(recording.wholes, ["abcd"]);
}

#[test]
fn writes_what_the_relay_held_back_when_the_output_pauses() {
    let mut recording = Recording::default();

    let (written, _) = relayed(
        "echo a; sleep 0.8; echo b",
        Errors::Joined,
        pace(50, 200, 1_024),
        &mut recording,
    );

    assert_eq!(written, "A\n<rest>B\n");
    assert!(recording.rests >= 2);
}

#[test]
fn joins_the_errors_to_the_output_or_leaves_them_where_they_went() {
    let script = "echo out; echo err 1>&2";
    let mut joined = Recording::default();
    let mut apart = Recording::default();

    relayed(
        script,
        Errors::Joined,
        pace(5_000, 1_000, 1_024),
        &mut joined,
    );
    relayed(script, Errors::Apart, pace(5_000, 1_000, 1_024), &mut apart);

    assert_eq!(joined.wholes, ["out\nerr\n"]);
    assert_eq!(apart.wholes, ["out\n"]);
}

#[test]
fn a_program_that_prints_nothing_writes_nothing() {
    let mut recording = Recording::default();

    let (written, ended) = relayed(
        "exit 7",
        Errors::Joined,
        pace(5_000, 1_000, 1_024),
        &mut recording,
    );

    assert_eq!(written, "");
    assert_eq!(recording.wholes, [""]);
    assert_eq!(ended.status.code(), Some(7));
}

#[test]
fn stops_the_program_when_its_output_cannot_be_written() {
    let started = Instant::now();

    let failed = relay(
        shell("echo a; sleep 30"),
        Errors::Joined,
        pace(50, 1_000, 1_024),
        &mut Recording::default(),
        &mut Closed,
    );

    assert_eq!(failed.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn says_when_the_program_cannot_start() {
    let failed = relay(
        Command::new("/nowhere/no-such-program"),
        Errors::Joined,
        pace(50, 50, 1_024),
        &mut Recording::default(),
        &mut Vec::new(),
    );

    assert_eq!(failed.unwrap_err().kind(), io::ErrorKind::NotFound);
}

#[test]
fn reads_a_large_output_by_pieces_of_up_to_64_kibibytes_and_never_passes_on_an_empty_one() {
    let mut recording = Recording::default();
    let script = "head -c 300000 /dev/zero | tr '\\0' 'x'; sleep 0.5; printf 'end'";

    let (written, ended) = relayed(script, Errors::Joined, pace(150, 5_000, 4), &mut recording);

    assert!(ended.status.success());
    assert_eq!(written.len(), 300_003 + "<rest>".len());
    assert!(recording.pieces.iter().any(|piece| piece.len() > 1_088));
    assert!(
        recording
            .pieces
            .iter()
            .all(|piece| !piece.is_empty() && piece.len() <= 65_536)
    );
}

#[test]
fn does_not_pass_on_an_empty_piece_when_the_hold_ends_with_nothing_held() {
    let mut recording = Recording::default();

    let (written, _) = relayed(
        "sleep 0.4; echo late",
        Errors::Joined,
        pace(100, 5_000, 1_024),
        &mut recording,
    );

    assert_eq!(recording.pieces, ["late\n"]);
    assert_eq!(written, "LATE\n<rest>");
}

#[test]
fn ends_with_the_program_and_hands_back_the_output_its_background_holds_open() {
    let mut recording = Recording::default();
    let started = Instant::now();

    let (written, ended) = relayed(
        "echo started; (sleep 2; echo late) & exit 4",
        Errors::Joined,
        pace(60_000, 5_000, 1_024),
        &mut recording,
    );
    let returned_after = started.elapsed();
    let mut late = String::new();
    ended
        .left_open
        .unwrap()
        .take(1_024)
        .read_to_string(&mut late)
        .unwrap();

    assert_eq!(recording.wholes, ["started\n"]);
    assert_eq!(written, "started\n");
    assert_eq!(ended.status.code(), Some(4));
    assert!(returned_after < Duration::from_secs(1));
    assert_eq!(late, "late\n");
}

#[test]
fn tells_what_the_relay_held_back_when_a_lasting_program_ends_with_its_output_open() {
    let mut recording = Recording::default();

    let (written, ended) = relayed(
        "echo a; sleep 0.4; echo b; sleep 1 &",
        Errors::Joined,
        pace(100, 5_000, 1_024),
        &mut recording,
    );

    assert_eq!(recording.pieces, ["a\n", "b\n"]);
    assert_eq!(written, "A\nB\n<rest>");
    assert!(ended.left_open.is_some());
}

#[test]
fn reads_what_was_written_before_the_end_and_says_whether_the_output_closed() {
    for closes in [false, true] {
        let (pipe, mut writer) = io::pipe().unwrap();
        writer.write_all(b"abc").unwrap();
        let still_open = (!closes).then_some(writer);
        let mut output = Output::new(pipe, PIECE);
        let (mut recording, mut written) = (Recording::default(), Vec::new());
        let mut flow = Flow::new(&mut recording, &mut written, 1_024);

        let closed = drain(&mut output, &mut flow).unwrap();
        flow.end().unwrap();

        assert_eq!(closed, closes);
        assert_eq!(recording.wholes, ["abc"]);
        drop(still_open);
    }
}

#[test]
fn reads_sixteen_pieces_at_most_once_the_program_has_ended() {
    let (pipe, mut writer) = io::pipe().unwrap();
    writer.write_all(&[b'x'; 40]).unwrap();
    let mut output = Output::new(pipe, 1);
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);

    let closed = drain(&mut output, &mut flow).unwrap();
    flow.end().unwrap();

    assert!(!closed);
    assert_eq!(recording.wholes, ["x".repeat(16)]);
}
