use super::*;

#[test]
fn frames_a_piece_with_its_length_in_front() {
    assert_eq!(frame(b"abc"), [0, 0, 0, 3, b'a', b'b', b'c']);
    assert_eq!(frame(b""), [0, 0, 0, 0]);
}

#[test]
fn frames_only_the_last_64_kib_of_a_longer_piece() {
    let mut piece = vec![b'a'; 10];
    piece.extend(vec![b'b'; MOST_FRAME]);

    let framed = frame(&piece);

    assert_eq!(framed.len(), LENGTH_BYTES + MOST_FRAME);
    assert_eq!(&framed[..LENGTH_BYTES], &[0, 1, 0, 0]);
    assert!(framed[LENGTH_BYTES..].iter().all(|&byte| byte == b'b'));
}

#[test]
fn names_the_process_then_the_command_in_one_frame() {
    let header = named(0x0102_0304, "ls");

    assert_eq!(header, [0, 0, 0, 6, 1, 2, 3, 4, b'l', b's']);
}

#[test]
fn keeps_the_process_of_a_name_too_long_and_cuts_the_name() {
    let command = "x".repeat(MOST_FRAME * 2);

    let header = named(9, &command);

    assert_eq!(header.len(), LENGTH_BYTES + MOST_FRAME);
    assert_eq!(
        &header[LENGTH_BYTES..LENGTH_BYTES + PROCESS_BYTES],
        &[0, 0, 0, 9]
    );
    assert!(
        header[LENGTH_BYTES + PROCESS_BYTES..]
            .iter()
            .all(|&byte| byte == b'x')
    );
}
