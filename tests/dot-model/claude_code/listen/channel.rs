use std::sync::mpsc;

use super::*;

#[test]
fn hands_each_whole_line_over_then_the_end() {
    let (sender, receiver) = mpsc::channel();

    read_lines(&b"one\ntwo\n"[..], &sender);

    let arrived: Vec<String> = receiver
        .try_iter()
        .map(|incoming| match incoming {
            Incoming::Line(line) => String::from_utf8(line).unwrap(),
            Incoming::Ended(Ok(())) => "end".to_owned(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(arrived, ["one", "two", "end"]);
}

#[test]
fn skips_a_line_longer_than_it_keeps() {
    let (sender, receiver) = mpsc::channel();
    let mut bytes = vec![b'x'; LONGEST_LINE + 1];
    bytes.extend_from_slice(b"\nkept\n");

    read_lines(bytes.as_slice(), &sender);

    let lines: Vec<Vec<u8>> = receiver
        .try_iter()
        .filter_map(|incoming| match incoming {
            Incoming::Line(line) => Some(line),
            _ => None,
        })
        .collect();
    assert_eq!(lines, [b"kept".to_vec()]);
}

#[test]
fn stops_reading_once_nobody_listens() {
    let (sender, receiver) = mpsc::channel();
    drop(receiver);

    read_lines(&b"one\ntwo\n"[..], &sender);
}

#[test]
fn turns_a_live_piece_into_what_the_listener_reads() {
    let piece = LivePiece {
        connection: 3,
        command: "ls",
        bytes: b"a\n",
    };

    let Incoming::Piece {
        connection,
        command,
        bytes,
    } = piece_of(piece)
    else {
        panic!("not a piece");
    };

    assert_eq!(
        (connection, command.as_str(), bytes.as_slice()),
        (3, "ls", &b"a\n"[..])
    );
}
