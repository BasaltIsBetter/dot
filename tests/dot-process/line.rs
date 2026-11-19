use std::io::{BufReader, Cursor};

use proptest::prelude::*;

use super::*;

const LONGEST: usize = 16;

fn lines_of(bytes: &[u8], capacity: usize, keep: usize) -> Vec<(Line, Vec<u8>)> {
    let mut reader = BufReader::with_capacity(capacity, Cursor::new(bytes.to_vec()));
    let mut kept = Vec::new();
    let mut lines = Vec::new();
    for _ in 0..=bytes.len() {
        let Some(line) = read_line(&mut reader, &mut kept, keep).unwrap() else {
            return lines;
        };
        lines.push((line, kept.clone()));
    }
    panic!("more lines than the {} bytes read", bytes.len());
}

#[test]
fn splits_lines_and_drops_their_ends() {
    let lines = lines_of(b"first\nsecond\r\n\nlast", 64, LONGEST);

    assert_eq!(
        lines,
        [
            (Line::Whole, b"first".to_vec()),
            (Line::Whole, b"second".to_vec()),
            (Line::Whole, Vec::new()),
            (Line::Whole, b"last".to_vec()),
        ]
    );
}

#[test]
fn reads_a_line_across_several_buffer_fills() {
    let lines = lines_of(b"a line longer than the buffer\n", 4, 64);

    assert_eq!(
        lines,
        [(Line::Whole, b"a line longer than the buffer".to_vec())]
    );
}

#[test]
fn joins_a_line_break_split_between_two_buffer_fills() {
    let lines = lines_of(b"abc\r\nd", 4, LONGEST);

    assert_eq!(
        lines,
        [(Line::Whole, b"abc".to_vec()), (Line::Whole, b"d".to_vec())]
    );
}

#[test]
fn keeps_only_the_start_of_a_line_too_long_and_goes_on() {
    let lines = lines_of(b"0123456789abcdefXYZ\nnext\n0123456789abcdefg", 4, LONGEST);

    assert_eq!(
        lines,
        [
            (Line::Cut, b"0123456789abcdef".to_vec()),
            (Line::Whole, b"next".to_vec()),
            (Line::Cut, b"0123456789abcdef".to_vec()),
        ]
    );
}

#[test]
fn keeps_a_line_exactly_as_long_as_allowed() {
    let lines = lines_of(b"0123456789abcdef\n", 4, LONGEST);

    assert_eq!(lines, [(Line::Whole, b"0123456789abcdef".to_vec())]);
}

#[test]
fn keeps_nothing_of_a_line_when_asked_for_nothing() {
    let lines = lines_of(b"abc\n\n", 4, 0);

    assert_eq!(lines, [(Line::Cut, Vec::new()), (Line::Whole, Vec::new())]);
}

#[test]
fn drops_one_carriage_return_at_the_end_only() {
    let lines = lines_of(b"a\rb\r\r\nlast\r", 2, LONGEST);

    assert_eq!(
        lines,
        [
            (Line::Whole, b"a\rb\r".to_vec()),
            (Line::Whole, b"last".to_vec()),
        ]
    );
}

#[test]
fn leaves_the_end_of_a_cut_line_alone() {
    let lines = lines_of(b"abc\r\n", 64, 4);
    let cut = lines_of(b"abcd\r\n", 64, 4);

    assert_eq!(lines, [(Line::Whole, b"abc".to_vec())]);
    assert_eq!(cut, [(Line::Cut, b"abcd".to_vec())]);
}

#[test]
fn ends_at_once_on_nothing() {
    assert_eq!(lines_of(b"", 4, LONGEST), []);
}

#[test]
fn counts_lines_as_an_editor_shows_them() {
    assert_eq!(line_count(b""), 0);
    assert_eq!(line_count(b"a"), 1);
    assert_eq!(line_count(b"a\n"), 1);
    assert_eq!(line_count(b"a\nb"), 2);
    assert_eq!(line_count(b"a\nb\n"), 2);
    assert_eq!(line_count(b"\n"), 1);
    assert_eq!(line_count(b"\n\n"), 2);
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
            .map(|line| (Line::Whole, line.as_bytes().to_vec()))
            .collect();
        prop_assert_eq!(read, expected);
    }

    #[test]
    fn never_keeps_more_than_asked_and_finds_every_line(
        bytes in proptest::collection::vec(any::<u8>(), 0..400),
        keep in 0_usize..20,
        capacity in 1_usize..64,
    ) {
        let lines = lines_of(&bytes, capacity, keep);

        for (_, line) in &lines {
            prop_assert!(line.len() <= keep);
            prop_assert!(!line.contains(&b'\n'));
        }
        prop_assert_eq!(u64::try_from(lines.len()).unwrap(), line_count(&bytes));
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
    let mut kept = Vec::new();

    assert_eq!(
        read_line(&mut reader, &mut kept, LONGEST).unwrap(),
        Some(Line::Whole)
    );
    assert_eq!(kept, b"line");
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
