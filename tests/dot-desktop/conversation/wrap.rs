use proptest::prelude::*;

use super::*;

fn rows_of(text: &str, width: usize) -> Vec<&str> {
    wrap(text, width)
        .into_iter()
        .map(|row| &text[row])
        .collect()
}

fn columns(text: &str) -> usize {
    text.chars().count()
}

#[test]
fn an_empty_text_still_has_one_empty_row() {
    assert_eq!(wrap("", 10), vec![0..0]);
}

#[test]
fn keeps_a_short_line_whole() {
    assert_eq!(rows_of("hello", 10), vec!["hello"]);
}

#[test]
fn keeps_a_line_exactly_as_wide_as_the_row() {
    assert_eq!(rows_of("0123456789", 10), vec!["0123456789"]);
}

#[test]
fn lets_trailing_spaces_hang_past_a_full_row() {
    assert_eq!(rows_of("0123456789 ", 10), vec!["0123456789 "]);
    assert_eq!(rows_of("0123456789   ", 10), vec!["0123456789   "]);
    assert_eq!(rows_of("abcd efgh ijkl", 9), vec!["abcd efgh ", "ijkl"]);
    assert_eq!(rows_of("abcd efgh   ijkl", 9), vec!["abcd efgh   ", "ijkl"]);
}

#[test]
fn breaks_after_the_last_space_that_fits() {
    assert_eq!(
        rows_of("hello world again", 12),
        vec!["hello world ", "again"]
    );
}

#[test]
fn starts_the_next_row_on_the_word_that_did_not_fit() {
    assert_eq!(rows_of("abcd efghij", 6), vec!["abcd ", "efghij"]);
}

#[test]
fn cuts_a_word_longer_than_the_row() {
    assert_eq!(
        rows_of("abcdefghijklmnop", 6),
        vec!["abcdef", "ghijkl", "mnop"]
    );
}

#[test]
fn lets_a_run_of_spaces_wider_than_the_row_hang() {
    assert_eq!(rows_of("ab          cd", 5), vec!["ab          ", "cd"]);
    assert_eq!(rows_of("          ", 5), vec!["          "]);
}

#[test]
fn starts_a_long_word_on_its_own_row_after_leading_spaces() {
    assert_eq!(rows_of("  abcdefgh", 4), vec!["  ", "abcd", "efgh"]);
}

#[test]
fn starts_a_row_at_each_new_line_and_keeps_empty_lines() {
    assert_eq!(rows_of("one\n\ntwo\n", 10), vec!["one", "", "two", ""]);
    assert_eq!(wrap("one\n\ntwo\n", 10), vec![0..3, 4..4, 5..8, 9..9]);
}

#[test]
fn counts_columns_in_characters_not_bytes() {
    assert_eq!(rows_of("éàçùô ïöüÿ", 5), vec!["éàçùô ", "ïöüÿ"]);
}

#[test]
fn treats_a_zero_width_as_one_column() {
    assert_eq!(rows_of("abc", 0), vec!["a", "b", "c"]);
}

proptest! {
    #[test]
    fn rows_cover_the_text_in_order_and_fit_the_width(
        text in "[a-c é\n]{0,200}",
        width in 1_usize..20,
    ) {
        let rows = wrap(&text, width);

        prop_assert_eq!(rows[0].start, 0);
        prop_assert_eq!(rows[rows.len() - 1].end, text.len());
        prop_assert_eq!(rows.len() > text.matches('\n').count(), true);
        for row in &rows {
            let shown = &text[row.clone()];
            prop_assert!(!shown.contains('\n'));
            prop_assert!(columns(shown.trim_end_matches(' ')) <= width, "{shown:?}");
        }
        for pair in rows.windows(2) {
            let gap = &text[pair[0].end..pair[1].start];
            prop_assert!(gap.is_empty() || gap == "\n", "gap {gap:?}");
        }
    }

    #[test]
    fn breaks_a_line_only_when_the_next_word_does_not_fit(
        words in prop::collection::vec(("[a-c]{1,8}", " {1,3}"), 0..30),
        width in 1_usize..20,
    ) {
        let text: String = words.iter().flat_map(|(word, gap)| [word.as_str(), gap.as_str()]).collect();
        let rows = wrap(&text, width);

        for pair in rows.windows(2) {
            let next_word = text[pair[1].clone()].split(' ').next().unwrap_or_default();
            prop_assert!(
                columns(&text[pair[0].clone()]) + columns(next_word) > width,
                "{text:?} broke before {next_word:?}"
            );
        }
    }
}
