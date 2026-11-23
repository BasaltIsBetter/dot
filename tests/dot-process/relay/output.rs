use std::io::Write;

use super::*;

struct Stubborn {
    interruptions: usize,
    then: io::ErrorKind,
}

impl Read for Stubborn {
    fn read(&mut self, piece: &mut [u8]) -> io::Result<usize> {
        if self.interruptions > 0 {
            self.interruptions -= 1;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.then == io::ErrorKind::Other {
            piece[..2].copy_from_slice(b"ok");
            return Ok(2);
        }
        Err(self.then.into())
    }
}

fn heard(output: &mut Output, wait: u64) -> Option<Vec<u8>> {
    match output.hear(Duration::from_millis(wait)).unwrap() {
        Heard::Piece(piece) => Some(piece.to_vec()),
        Heard::Nothing => Some(Vec::new()),
        Heard::Closed => None,
    }
}

#[test]
fn hears_nothing_from_an_output_nobody_wrote_to_and_does_not_wait_longer_than_asked() {
    let (pipe, writer) = io::pipe().unwrap();
    let mut output = Output::new(pipe, 64);
    let started = std::time::Instant::now();

    let at_once = heard(&mut output, 0);
    let after_a_wait = heard(&mut output, 50);
    let waited = started.elapsed();

    assert_eq!(at_once, Some(Vec::new()));
    assert_eq!(after_a_wait, Some(Vec::new()));
    assert!(waited >= Duration::from_millis(40));
    assert!(waited < Duration::from_secs(5));
    drop(writer);
}

#[test]
fn hears_what_was_written_in_pieces_no_longer_than_asked() {
    let (pipe, mut writer) = io::pipe().unwrap();
    writer.write_all(b"abcdef").unwrap();
    let mut output = Output::new(pipe, 4);

    let pieces = [
        heard(&mut output, 0),
        heard(&mut output, 0),
        heard(&mut output, 0),
    ];

    assert_eq!(
        pieces,
        [
            Some(b"abcd".to_vec()),
            Some(b"ef".to_vec()),
            Some(Vec::new())
        ]
    );
}

#[test]
fn hears_that_the_output_closed_after_what_was_written() {
    let (pipe, mut writer) = io::pipe().unwrap();
    writer.write_all(b"last").unwrap();
    drop(writer);
    let mut output = Output::new(pipe, 64);

    let pieces = [heard(&mut output, 0), heard(&mut output, 0)];

    assert_eq!(pieces, [Some(b"last".to_vec()), None]);
}

#[test]
fn hands_the_pipe_back_with_what_was_not_read() {
    let (pipe, mut writer) = io::pipe().unwrap();
    writer.write_all(b"abcdef").unwrap();
    drop(writer);
    let mut output = Output::new(pipe, 4);
    heard(&mut output, 0);

    let mut left = Vec::new();
    output.into_pipe().take(64).read_to_end(&mut left).unwrap();

    assert_eq!(left, b"ef");
}

#[test]
fn tries_an_interrupted_read_again_and_returns_any_other_error() {
    let mut piece = [0; 8];
    let mut interrupted = Stubborn {
        interruptions: 2,
        then: io::ErrorKind::Other,
    };
    let mut broken = Stubborn {
        interruptions: 1,
        then: io::ErrorKind::BrokenPipe,
    };

    let read = read_piece(&mut interrupted, &mut piece).unwrap();
    let failed = read_piece(&mut broken, &mut piece);

    assert_eq!(&piece[..read], b"ok");
    assert_eq!(failed.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
}
