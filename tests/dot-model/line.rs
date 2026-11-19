use std::io::{BufReader, Cursor};

use proptest::prelude::*;

use super::*;

const LONGEST: usize = 16;

fn lines_of(bytes: &[u8], capacity: usize, longest: usize) -> Vec<(Read, Vec<u8>)> {
    let mut reader = BufReader::with_capacity(capacity, Cursor::new(bytes.to_vec()));
    let mut line = Vec::new();
    let mut lines = Vec::new();
    for _ in 0..=bytes.len() {
        let read = read_line(&mut reader, &mut line, longest).unwrap();
        if read == Read::End {
            return lines;
        }
        lines.push((read, line.clone()));
    }
    panic!("more lines than the {} bytes read", bytes.len());
}

#[test]
fn splits_lines_and_drops_their_ends() {
    let lines = lines_of(b"first\nsecond\r\n\nlast", 64, LONGEST);

    assert_eq!(
        lines,
        [
            (Read::Line, b"first".to_vec()),
            (Read::Line, b"second".to_vec()),
            (Read::Line, Vec::new()),
            (Read::Line, b"last".to_vec()),
        ]
    );
}

#[test]
fn reads_a_line_across_several_buffer_fills() {
    let lines = lines_of(b"a line longer than the buffer\n", 4, 64);

    assert_eq!(
        lines,
        [(Read::Line, b"a line longer than the buffer".to_vec())]
    );
}

#[test]
fn skips_a_line_too_long_and_goes_on() {
    let lines = lines_of(b"0123456789abcdefXYZ\nnext\n0123456789abcdefg", 4, LONGEST);

    assert_eq!(
        lines,
        [
            (Read::TooLong, Vec::new()),
            (Read::Line, b"next".to_vec()),
            (Read::TooLong, Vec::new()),
        ]
    );
}

#[test]
fn keeps_a_line_exactly_as_long_as_allowed() {
    let lines = lines_of(b"0123456789abcdef\n", 4, LONGEST);

    assert_eq!(lines, [(Read::Line, b"0123456789abcdef".to_vec())]);
}

#[test]
fn ends_at_once_on_nothing() {
    assert_eq!(lines_of(b"", 4, LONGEST), []);
}

proptest! {
    #[test]
    fn gives_back_every_short_line(
        lines in proptest::collection::vec("[ -~]{0,16}", 0..20),
        capacity in 1_usize..32,
    ) {
        let text: String = lines.iter().flat_map(|line| [line.as_str(), "\n"]).collect();

        let read = lines_of(text.as_bytes(), capacity, LONGEST);

        let expected: Vec<_> = lines
            .iter()
            .map(|line| (Read::Line, line.as_bytes().to_vec()))
            .collect();
        prop_assert_eq!(read, expected);
    }

    #[test]
    fn never_keeps_more_than_allowed(
        bytes in proptest::collection::vec(any::<u8>(), 0..400),
        capacity in 1_usize..64,
    ) {
        for (_, line) in lines_of(&bytes, capacity, LONGEST) {
            prop_assert!(line.len() <= LONGEST);
            prop_assert!(!line.contains(&b'\n'));
        }
    }
}

struct Stuttering {
    failures: Vec<std::io::ErrorKind>,
    rest: Cursor<Vec<u8>>,
}

impl std::io::Read for Stuttering {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(kind) = self.failures.pop() {
            return Err(std::io::Error::from(kind));
        }
        self.rest.read(buffer)
    }
}

#[test]
fn reads_on_after_an_interrupted_read() {
    let reader = Stuttering {
        failures: vec![std::io::ErrorKind::Interrupted],
        rest: Cursor::new(b"line\n".to_vec()),
    };
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();

    assert_eq!(
        read_line(&mut reader, &mut line, LONGEST).unwrap(),
        Read::Line
    );
    assert_eq!(line, b"line");
}

#[test]
fn reports_any_other_failed_read() {
    let reader = Stuttering {
        failures: vec![std::io::ErrorKind::ConnectionReset],
        rest: Cursor::new(b"line\n".to_vec()),
    };
    let mut reader = BufReader::new(reader);

    let error = read_line(&mut reader, &mut Vec::new(), LONGEST).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
}
