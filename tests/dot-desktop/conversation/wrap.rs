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

#[test]
fn cleans_tabs_line_ends_and_control_characters() {
    assert_eq!(
        normalized("a\tb\r\nc\rd\u{1b}e", usize::MAX),
        "a    b\nc\nde"
    );
    assert_eq!(normalized("abcdef", 3), "abc");
    assert_eq!(normalized("\u{7}", usize::MAX), "");
}

fn wrap_counting_the_whole_rest(text: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0;
    for line in text.split('\n') {
        let mut start = 0;
        loop {
            let rest = &line[start..];
            if columns(rest.trim_end_matches(' ')) <= width {
                rows.push(line_start + start..line_start + line.len());
                break;
            }
            let end = row_end(rest, width);
            rows.push(line_start + start..line_start + start + end);
            start += end;
        }
        line_start += line.len() + 1;
    }
    rows
}

proptest! {
    #[test]
    fn wraps_exactly_like_counting_the_whole_rest_of_each_line(
        text in "[a-c é\n]{0,300}",
        width in 1_usize..24,
    ) {
        prop_assert_eq!(wrap(&text, width), wrap_counting_the_whole_rest(&text, width));
    }
}

#[test]
fn wraps_one_long_line_with_trailing_spaces_like_counting_the_whole_rest() {
    let text = format!("{}{}", "abc ".repeat(500), " ".repeat(300));

    assert_eq!(wrap(&text, 17), wrap_counting_the_whole_rest(&text, 17));
}

fn cut_rows_of(text: &str, width: usize) -> Vec<&str> {
    wrap_characters(text, width)
        .into_iter()
        .map(|row| &text[row])
        .collect()
}

#[test]
fn cuts_a_word_where_the_row_ends_instead_of_moving_it_down() {
    assert_eq!(cut_rows_of("abc defgh", 6), vec!["abc de", "fgh"]);
}

#[test]
fn keeps_a_line_as_wide_as_the_row_on_one_row_when_cutting() {
    assert_eq!(cut_rows_of("abcdef", 6), vec!["abcdef"]);
    assert_eq!(cut_rows_of("abcdefg", 6), vec!["abcdef", "g"]);
}

#[test]
fn cuts_spaces_like_any_other_character() {
    assert_eq!(cut_rows_of("abcdef   ", 6), vec!["abcdef", "   "]);
}

#[test]
fn cuts_at_each_new_line_and_keeps_empty_lines() {
    assert_eq!(cut_rows_of("", 6), vec![""]);
    assert_eq!(cut_rows_of("ab\n\ncd", 6), vec!["ab", "", "cd"]);
    assert_eq!(cut_rows_of("abc", 0), vec!["a", "b", "c"]);
}

#[test]
fn cuts_in_characters_not_bytes() {
    assert_eq!(cut_rows_of("éèàç€", 2), vec!["éè", "àç", "€"]);
}

proptest! {
    #[test]
    fn cut_rows_cover_the_text_and_fill_every_row_but_the_last_of_a_line(
        text in "[a-zé €\n]{0,200}",
        width in 1_usize..20,
    ) {
        let rows = wrap_characters(&text, width);

        let mut covered = 0;
        for (index, row) in rows.iter().enumerate() {
            prop_assert!(row.start == covered || row.start == covered + 1);
            covered = row.end;
            let count = columns(&text[row.clone()]);
            let ends_a_line = rows.get(index + 1).is_none_or(|next| next.start != row.end);
            prop_assert!(count <= width);
            prop_assert!(ends_a_line || count == width);
            prop_assert!(!text[row.clone()].contains('\n'));
        }
        prop_assert_eq!(covered, text.len());
    }
}

#[test]
fn cuts_the_draft_counting_a_tab_as_four_columns() {
    let cut = |text: &str, width| {
        wrap_characters(text, width)
            .into_iter()
            .map(|row| text[row].to_owned())
            .collect::<Vec<_>>()
    };

    assert_eq!(cut("ab\tcd", 6), ["ab\t", "cd"]);
    assert_eq!(cut("abc\tx", 6), ["abc", "\tx"]);
    assert_eq!(cut("abcdefgh", 4), ["abcd", "efgh"]);
}

#[test]
fn keeps_tabs_in_typed_text_and_shows_them_as_four_spaces() {
    assert_eq!(typed("a\tb\r\nc\rd\u{1b}e", usize::MAX), "a\tb\nc\nde");
    assert_eq!(typed("abcdef", 3), "abc");
    assert_eq!(shown("\tx"), "    x");
    assert!(matches!(shown("x"), std::borrow::Cow::Borrowed("x")));
    assert_eq!(cells('\t'), 4);
    assert_eq!(cells('é'), 1);
}

fn cut_by_characters(text: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0;
    for line in text.split('\n') {
        let mut start = 0;
        let mut used = 0;
        for (index, character) in line.char_indices() {
            let wide = cells(character);
            if used > 0 && used + wide > width {
                rows.push(line_start + start..line_start + index);
                start = index;
                used = 0;
            }
            used += wide;
        }
        rows.push(line_start + start..line_start + line.len());
        line_start += line.len() + 1;
    }
    rows
}

proptest! {
    #[test]
    fn cuts_the_draft_by_bytes_like_counting_cells_by_characters(
        text in "[a-c é漢\t\n]{0,80}",
        width in 1_usize..12,
    ) {
        prop_assert_eq!(wrap_characters(&text, width), cut_by_characters(&text, width));
    }
}
