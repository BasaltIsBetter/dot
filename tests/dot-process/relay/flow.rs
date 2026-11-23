use super::*;
use crate::relay::tests::{Closed, Recording};

#[test]
fn holds_pieces_until_they_exceed_what_it_may_hold() {
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 4);

    flow.take(b"ab").unwrap();
    flow.take(b"cd").unwrap();
    let held_at_four = flow.is_holding();
    flow.take(b"e").unwrap();
    let held_at_five = flow.is_holding();
    flow.take(b"f").unwrap();

    assert!(held_at_four);
    assert!(!held_at_five);
    assert_eq!(recording.pieces, ["abcde", "f"]);
    assert_eq!(written, b"ABCDEF");
}

#[test]
fn lets_go_of_what_it_held_as_one_piece_and_of_nothing_as_no_piece() {
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    flow.take(b"held").unwrap();
    flow.let_go().unwrap();
    let still_holding = flow.is_holding();
    flow.let_go().unwrap();

    assert!(!still_holding);
    assert_eq!(recording.pieces, ["held"]);
    assert_eq!(written, b"HELD");
}

#[test]
fn ends_a_held_output_whole_as_it_is_or_as_the_relay_shortens_it() {
    for replaces in [false, true] {
        let mut recording = Recording {
            replaces,
            ..Recording::default()
        };
        let mut written = Vec::new();
        let mut flow = Flow::new(&mut recording, &mut written, 1_024);
        flow.take(b"all of it").unwrap();

        flow.end().unwrap();

        assert_eq!(recording.wholes, ["all of it"]);
        assert_eq!(recording.rests, 0);
        let expected: &[u8] = if replaces { b"<whole>" } else { b"all of it" };
        assert_eq!(written, expected);
    }
}

#[test]
fn ends_a_passing_output_with_what_the_relay_held_back() {
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);
    flow.let_go().unwrap();

    flow.end().unwrap();

    assert_eq!(recording.wholes, [""; 0]);
    assert_eq!(written, b"<rest>");
}

#[test]
fn writes_what_the_relay_held_back_at_a_pause() {
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 1_024);

    flow.pause().unwrap();
    flow.pause().unwrap();

    assert_eq!(recording.rests, 2);
    assert_eq!(written, b"<rest>");
}

#[test]
fn does_not_touch_the_writer_for_nothing_and_returns_its_error_otherwise() {
    let mut recording = Recording {
        rests: 1,
        ..Recording::default()
    };
    let mut closed = Closed;
    let mut flow = Flow::new(&mut recording, &mut closed, 1_024);

    let nothing = flow.pause();
    let something = flow.take(b"more than it may hold".repeat(100).as_slice());

    assert!(nothing.is_ok());
    assert_eq!(something.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
}

struct Parting {
    rests: usize,
}

impl Relay for Parting {
    fn whole(&mut self, _: &[u8]) -> Option<Vec<u8>> {
        Some(b"<whole>".to_vec())
    }

    fn piece(&mut self, piece: &[u8]) -> Vec<u8> {
        piece.to_vec()
    }

    fn rest(&mut self) -> Vec<u8> {
        self.rests += 1;
        b"<rest>".to_vec()
    }

    fn last(&mut self) -> Vec<u8> {
        b"<last>".to_vec()
    }
}

#[test]
fn asks_the_relay_for_its_last_word_at_the_end_of_a_passing_output_only() {
    let (mut passing, mut held) = (Parting { rests: 0 }, Parting { rests: 0 });
    let (mut written, mut written_whole) = (Vec::new(), Vec::new());
    let mut flow = Flow::new(&mut passing, &mut written, 1_024);
    flow.let_go().unwrap();
    flow.pause().unwrap();
    flow.end().unwrap();
    Flow::new(&mut held, &mut written_whole, 1_024)
        .end()
        .unwrap();

    assert_eq!(written, b"<rest><last>");
    assert_eq!(passing.rests, 1);
    assert_eq!(written_whole, b"<whole>");
    assert_eq!(held.rests, 0);
}

#[test]
fn lets_the_relay_hear_every_piece_as_read_held_or_not() {
    let (mut recording, mut written) = (Recording::default(), Vec::new());
    let mut flow = Flow::new(&mut recording, &mut written, 4);

    flow.take(b"ab").unwrap();
    flow.take(b"cde").unwrap();
    flow.take(b"f").unwrap();

    assert_eq!(recording.heard, ["ab", "cde", "f"]);
    assert_eq!(recording.pieces, ["abcde", "f"]);
}
