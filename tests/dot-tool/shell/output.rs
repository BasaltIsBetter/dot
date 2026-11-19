use super::*;

const SENTINEL: &str = "__DOT_FOLDER_test__";

fn captured(pieces: &[&[u8]]) -> Capture {
    let mut capture = Capture::default();
    for piece in pieces {
        capture.push(piece);
    }
    capture
}

#[test]
fn takes_the_folder_from_the_sentinel_line_and_leaves_it_out() {
    let capture = captured(&[b"hello\n", b"\n__DOT_FOLDER_test__/tmp/project\n"]);

    let heard = capture.heard(SENTINEL);

    assert_eq!(heard.text, "hello\n");
    assert_eq!(heard.folder, Some(PathBuf::from("/tmp/project")));
}

#[test]
fn an_output_without_the_sentinel_keeps_everything_and_no_folder() {
    let heard = captured(&[b"no sentinel here"]).heard(SENTINEL);

    assert_eq!(heard.text, "no sentinel here");
    assert_eq!(heard.folder, None);
}

#[test]
fn takes_the_last_sentinel_when_the_command_printed_one_too() {
    let heard = captured(&[
        b"\n__DOT_FOLDER_test__/fake\nmore\n",
        b"\n__DOT_FOLDER_test__/real\n",
    ])
    .heard(SENTINEL);

    assert_eq!(heard.text, "\n__DOT_FOLDER_test__/fake\nmore\n");
    assert_eq!(heard.folder, Some(PathBuf::from("/real")));
}

#[test]
fn keeps_the_head_and_the_tail_of_a_long_output() {
    let head = vec![b'h'; HEAD];
    let middle = vec![b'm'; 10];
    let tail = vec![b't'; TAIL];

    let long = captured(&[&head, &middle, &tail, b"\n__DOT_FOLDER_test__/x\n"]).heard(SENTINEL);

    let expected_tail = "t".repeat(TAIL - 23);
    assert!(long.text.starts_with(&"h".repeat(HEAD)));
    assert!(long.text.contains("[… 33 bytes of output cut here …]"));
    assert!(long.text.ends_with(&expected_tail));
    assert!(!long.text.contains('m'));
    assert_eq!(long.folder, Some(PathBuf::from("/x")));
}

#[test]
fn keeps_an_output_of_exactly_both_halves_whole() {
    let output = vec![b'a'; HEAD + TAIL];

    let heard = captured(&[&output]).heard(SENTINEL);

    assert_eq!(heard.text.len(), HEAD + TAIL);
    assert!(!heard.text.contains("cut here"));
}

#[test]
fn counts_every_byte_cut_across_many_pieces() {
    let mut capture = Capture::default();
    for _ in 0..100 {
        capture.push(&[b'x'; 1_000]);
    }

    let heard = capture.heard(SENTINEL);

    assert!(heard.text.contains("[… 70000 bytes of output cut here …]"));
}
