use super::*;

#[test]
fn counts_a_line_repeated_three_times_or_more() {
    assert_eq!(
        collapse("a\nretry\nretry\nretry\nb\n").as_deref(),
        Some("a\nretry [×3]\nb\n")
    );
}

#[test]
fn keeps_a_line_written_twice() {
    assert_eq!(collapse("a\nretry\nretry\nb\n"), None);
}

#[test]
fn keeps_every_blank_line_of_a_run() {
    assert_eq!(collapse("a\n\n\n\nb"), None);
    assert_eq!(collapse("a\n  \n  \n  \nb"), None);
    assert_eq!(
        collapse("a\n\n\n\nretry\nretry\nretry\n").as_deref(),
        Some("a\n\n\n\nretry [×3]\n")
    );
}

#[test]
fn says_nothing_changed_when_no_line_repeats() {
    assert_eq!(collapse("a\nb\n\nc\n"), None);
    assert_eq!(collapse(""), None);
}
