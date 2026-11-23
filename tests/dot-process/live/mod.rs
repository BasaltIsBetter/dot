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
