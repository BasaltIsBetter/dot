use std::io::BufReader;

use proptest::prelude::*;

use super::*;

fn lines_of(bytes: &[u8], keep: usize, capacity: usize) -> Vec<(String, bool)> {
    let mut reader = BufReader::with_capacity(capacity, bytes);
    let mut kept = Vec::new();
    let mut lines = Vec::new();
    for _ in 0..=bytes.len() {
        let Some(cut) = read_line(&mut reader, &mut kept, keep).unwrap() else {
            return lines;
        };
        lines.push((String::from_utf8(kept.clone()).unwrap(), cut));
    }
    panic!("more lines than the {} bytes read", bytes.len());
}

#[test]
fn a_nul_byte_makes_a_file_binary() {
    assert!(is_binary(b"abc\0def"));
    assert!(!is_binary(b"abc def\n"));
    assert!(!is_binary(b""));
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

#[test]
fn recognizes_windows_line_endings_by_the_first_line() {
    assert!(uses_crlf(b"a\r\nb\n"));
    assert!(!uses_crlf(b"a\nb\r\n"));
    assert!(!uses_crlf(b"\nb"));
    assert!(!uses_crlf(b"no break"));
    assert!(!uses_crlf(b""));
}

#[test]
fn reads_lines_without_their_endings() {
    assert_eq!(
        lines_of(b"one\r\ntwo\nthree", 100, 4),
        [
            ("one".to_owned(), false),
            ("two".to_owned(), false),
            ("three".to_owned(), false)
        ]
    );
    assert_eq!(lines_of(b"", 100, 4), []);
    assert_eq!(lines_of(b"\n", 100, 4), [(String::new(), false)]);
}

#[test]
fn keeps_only_the_start_of_a_long_line_and_says_so() {
    assert_eq!(
        lines_of(b"abcdefgh\nij\n", 3, 2),
        [("abc".to_owned(), true), ("ij".to_owned(), false)]
    );
    assert_eq!(lines_of(b"abc\n", 3, 2), [("abc".to_owned(), false)]);
}

proptest! {
    #[test]
    fn never_keeps_more_than_asked_and_finds_every_line(
        text in "[a-z\\n\\r]{0,200}",
        keep in 0_usize..20,
        capacity in 1_usize..16,
    ) {
        let lines = lines_of(text.as_bytes(), keep, capacity);

        prop_assert!(lines.iter().all(|(line, _)| line.len() <= keep));
        prop_assert_eq!(u64::try_from(lines.len()).unwrap(), line_count(text.as_bytes()));
    }
}
