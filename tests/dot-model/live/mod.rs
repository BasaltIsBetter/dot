use std::borrow::Cow;

use dot_api::{Origin, OutputCompaction};

use super::*;
use crate::tool::tests::Untouched;

struct Shouting;

impl Compactor for Shouting {
    fn compact(&self, _: Origin<'_>, _: &str) -> Option<String> {
        None
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        Cow::Owned(output.to_uppercase())
    }

    fn tune(&mut self, _: OutputCompaction) {}
}

fn lines_after(pieces: &[&[u8]]) -> Vec<String> {
    let mut tail = LiveTail::default();
    for piece in pieces {
        tail.push(piece);
    }
    tail.lines(&Untouched)
}

#[test]
fn has_no_lines_before_anything_is_printed() {
    assert_eq!(lines_after(&[]), Vec::<String>::new());
}

#[test]
fn keeps_the_last_three_lines_oldest_first() {
    assert_eq!(lines_after(&[b"1\n2\n3\n4\n5\n"]), ["3", "4", "5"]);
}

#[test]
fn counts_the_line_still_growing_among_the_three() {
    assert_eq!(lines_after(&[b"1\n2\n3\nfour"]), ["2", "3", "four"]);
    assert_eq!(lines_after(&[b"1\n2\n3\nfo", b"ur"]), ["2", "3", "four"]);
}

#[test]
fn shows_a_line_redrawn_in_place_in_its_last_state_even_across_pieces() {
    assert_eq!(lines_after(&[b" 10%\r 50%\r", b"100%\n"]), ["100%"]);
    assert_eq!(lines_after(&[b"downloading 10%\r"]), ["downloading 10%"]);
}

#[test]
fn ends_a_line_at_a_carriage_return_and_line_break_even_across_pieces() {
    assert_eq!(lines_after(&[b"one\r", b"\ntwo\r\n"]), ["one", "two"]);
}

#[test]
fn keeps_the_first_kibibyte_of_a_long_line() {
    let long = vec![b'x'; 5_000];

    let lines = lines_after(&[&long, b"\n"]);

    assert_eq!(lines, ["x".repeat(1_024)]);
}

#[test]
fn hands_each_line_to_the_compactor_to_be_read_by_a_person() {
    let mut tail = LiveTail::default();
    tail.push(b"ok\nrunning");

    assert_eq!(tail.lines(&Shouting), ["OK", "RUNNING"]);
}
