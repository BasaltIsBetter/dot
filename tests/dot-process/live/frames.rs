use super::*;
use crate::live::{frame, named};

#[test]
fn reads_frames_split_anywhere() {
    let mut frames = Frames::default();
    let mut bytes = frame(b"one");
    bytes.extend(frame(b""));
    bytes.extend(frame(b"three"));

    let mut found = Vec::new();
    for byte in bytes {
        frames.extend(&[byte]);
        while let Some(Ok(frame)) = frames.next() {
            found.push(frame);
        }
    }

    assert_eq!(found, [b"one".to_vec(), Vec::new(), b"three".to_vec()]);
}

#[test]
fn tells_a_frame_too_long_apart_from_one_still_coming() {
    let mut frames = Frames::default();
    frames.extend(&65_536_u32.to_be_bytes());
    let coming = frames.next();
    let mut too_long = Frames::default();
    too_long.extend(&65_537_u32.to_be_bytes());

    assert!(coming.is_none());
    assert!(matches!(too_long.next(), Some(Err(TooLong))));
}

#[test]
fn reads_the_process_then_the_command_of_a_header() {
    let header = named(4_321, "cargo test");

    let read = Header::read(&header[LENGTH_BYTES..]);

    assert_eq!(
        read,
        Some(Header {
            process: 4_321,
            command: "cargo test".to_owned(),
        })
    );
}

#[test]
fn reads_a_header_with_an_empty_or_broken_name() {
    let empty = Header::read(&7_u32.to_be_bytes()).unwrap();
    let mut broken = 7_u32.to_be_bytes().to_vec();
    broken.extend([b'a', 0xff]);

    assert_eq!(empty.command, "");
    assert_eq!(Header::read(&broken).unwrap().command, "a\u{fffd}");
}

#[test]
fn refuses_a_header_too_short_for_a_process() {
    assert_eq!(Header::read(b""), None);
    assert_eq!(Header::read(b"abc"), None);
}
