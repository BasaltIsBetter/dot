use super::*;

#[test]
fn keeps_the_first_and_the_last_of_lines_that_only_differ_by_their_numbers() {
    let text = "start\nrequest 1 took 12 ms\nrequest 2 took 7 ms\nrequest 30 took 110 ms\nrequest 4 took 9 ms\nend\n";

    assert_eq!(
        lines(text).as_deref(),
        Some("start\nrequest 1 took 12 ms\n[+2 similar lines]\nrequest 4 took 9 ms\nend\n")
    );
}

#[test]
fn keeps_every_line_of_a_run_too_short_to_thin() {
    let text = "request 1\nrequest 2\nrequest 3\nother\nitem 1\nitem 2\nitem 3\nitem 4";

    assert_eq!(
        lines(text).as_deref(),
        Some("request 1\nrequest 2\nrequest 3\nother\nitem 1\n[+2 similar lines]\nitem 4")
    );
    assert_eq!(lines("a 1\na 2\na 3\n"), None);
}

#[test]
fn leaves_alone_different_words_blank_lines_and_nothing() {
    assert_eq!(lines("alpha 1\nbravo 2\ncharlie 3\ndelta 4\n"), None);
    assert_eq!(lines("\n\n\n\n\n"), None);
    assert_eq!(lines("  \n  \n  \n  \n"), None);
    assert_eq!(lines("item\nitem 2\nitem\nitem 4\n"), None);
    assert_eq!(lines(""), None);
}

#[test]
fn a_number_is_any_run_of_digits_wherever_it_stands() {
    assert!(are_similar("v1.20.3 at 08:15", "v10.2.33 at 23:59"));
    assert!(are_similar("", ""));
    assert!(are_similar("no number", "no number"));
    assert!(!are_similar("a1", "a"));
    assert!(!are_similar("1 a", "1 b"));
    assert!(!are_similar("a 1", "a 1 1"));
    assert!(!are_similar("a 1b", "a b1"));
}

#[test]
fn compares_what_stands_between_the_numbers_byte_for_byte() {
    assert!(are_similar("été 1", "été 2026"));
    assert!(are_similar("12", "1"));
    assert!(are_similar("1a22b333", "4444a5b6"));
    assert!(!are_similar("été 1", "ete 1"));
    assert!(!are_similar("1", "a"));
    assert!(!are_similar("a", "1"));
    assert!(!are_similar("a", "ab"));
    assert!(!are_similar("ab", "a"));
    assert!(!are_similar("a1", "a1b"));
    assert!(!are_similar("a1b", "a1"));
}

#[test]
fn a_number_one_line_has_and_the_other_has_not_makes_them_differ() {
    assert!(!are_similar("a1b", "ab"));
    assert!(!are_similar("ab", "a1b"));
    assert!(!are_similar("1ab", "ab"));
}
