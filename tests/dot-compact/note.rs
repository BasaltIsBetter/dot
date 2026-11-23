use super::*;

#[test]
fn counts_what_was_left_out_between_square_brackets() {
    assert_eq!(
        left_out(12, "lines: build progress"),
        "[+12 lines: build progress]"
    );
}

#[test]
fn ends_like_the_original_text() {
    assert_eq!(finish("a\nb\n".to_owned(), "x\ny"), "a\nb");
    assert_eq!(finish("a\nb\n".to_owned(), "x\ny\n"), "a\nb\n");
    assert_eq!(finish("a".to_owned(), "x"), "a");
}

#[test]
fn pushes_a_line_with_its_ending() {
    let mut text = String::from("a\n");

    push_line(&mut text, "b");

    assert_eq!(text, "a\nb\n");
}
