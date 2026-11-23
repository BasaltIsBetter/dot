use proptest::prelude::*;

use super::*;

#[test]
fn a_quiet_output_is_not_copied() {
    assert!(matches!(
        clean("plain\ntext\n"),
        Cow::Borrowed("plain\ntext\n")
    ));
}

#[test]
fn removes_colours_and_cursor_moves() {
    assert_eq!(
        clean("\u{1b}[1;31merror\u{1b}[0m: no\u{1b}[K\n"),
        "error: no\n"
    );
}

#[test]
fn removes_a_window_title_ended_by_a_bell_or_by_a_string_end() {
    assert_eq!(clean("\u{1b}]0;title\u{7}text"), "text");
    assert_eq!(clean("\u{1b}]8;;https://example.com\u{1b}\\link"), "link");
}

#[test]
fn keeps_only_what_a_progress_bar_drew_last() {
    assert_eq!(clean("10%\r50%\r100%\ndone\n"), "100%\ndone\n");
}

#[test]
fn reads_windows_line_endings() {
    assert_eq!(clean("a\r\nb\r\n"), "a\nb\n");
}

#[test]
fn an_escape_cut_short_removes_the_rest_of_its_line_only() {
    assert_eq!(clean("a\u{1b}[31\nb"), "a\nb");
    assert_eq!(clean("a\u{1b}"), "a");
    assert_eq!(clean("a\u{1b}]0;never ended\nb"), "a\nb");
}

#[test]
fn removes_the_choice_of_a_character_set_with_the_letter_that_ends_it() {
    assert_eq!(clean("\u{1b}[1mok\u{1b}(B\u{1b}[m done\n"), "ok done\n");
    assert_eq!(clean("a\u{1b}#8b\u{1b}7c\u{1b}%/Gd\n"), "abcd\n");
    assert_eq!(clean("a\u{1b}(\nb"), "a\nb");
}

#[test]
fn counts_what_later_drawings_of_an_unfinished_line_drew_over() {
    assert_eq!(drawn_over("10%\r20%\r30%"), 8);
    assert_eq!(drawn_over("10%\r20%\r"), 4);
    assert_eq!(drawn_over("10%\r"), 0);
    assert_eq!(drawn_over("10%"), 0);
    assert_eq!(drawn_over("\r\r"), 0);
    assert_eq!(drawn_over(""), 0);
}

proptest! {
    #[test]
    fn forgetting_what_was_drawn_over_changes_nothing_of_the_line(
        unfinished in "[a-c\r]{0,12}",
        then in "[a-c\r]{0,6}",
    ) {
        let kept = &unfinished[drawn_over(&unfinished)..];
        let shortened = format!("{kept}{then}\n");
        let whole = format!("{unfinished}{then}\n");

        prop_assert_eq!(clean(&shortened), clean(&whole));
    }

    #[test]
    fn nothing_of_the_terminal_is_left(output in any::<String>()) {
        let cleaned = clean(&output);

        prop_assert!(!cleaned.contains(ESCAPE));
        prop_assert!(!cleaned.contains(CARRIAGE_RETURN));
        prop_assert!(cleaned.len() <= output.len());
    }

    #[test]
    fn text_without_terminal_noise_stays_as_it_is(output in "[a-zA-Z0-9 \n:/.é-]{0,200}") {
        prop_assert_eq!(clean(&output), output.as_str());
    }
}

#[test]
fn looks_for_the_bytes_of_an_escape_and_of_a_carriage_return() {
    assert_eq!(NOISY_BYTES.map(char::from), [ESCAPE, CARRIAGE_RETURN]);
    assert!(matches!(
        clean("é ─ ✓ no noise in other bytes"),
        Cow::Borrowed(_)
    ));
    assert!(matches!(clean("a\rb"), Cow::Owned(_)));
    assert!(matches!(clean("\u{1b}[31mred"), Cow::Owned(_)));
}
