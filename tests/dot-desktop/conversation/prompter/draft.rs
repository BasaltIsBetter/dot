use proptest::prelude::*;

use super::*;

fn typed(text: &str) -> Draft {
    let mut draft = Draft::default();
    draft.apply(Edit::Insert(text.to_owned()));
    draft
}

fn after(draft: &mut Draft, edits: impl IntoIterator<Item = Edit>) {
    for edit in edits {
        draft.apply(edit);
    }
}

#[test]
fn starts_empty_with_the_caret_on_one_row() {
    let draft = Draft::default();

    assert_eq!(draft.text(), "");
    assert_eq!(draft.rows(), vec![0..0]);
    assert_eq!(draft.caret_cell(), (0, 0));
}

#[test]
fn types_at_the_caret() {
    let mut draft = typed("helo");

    after(
        &mut draft,
        [Edit::Move(Motion::Left), Edit::Insert("l".to_owned())],
    );

    assert_eq!(draft.text(), "hello");
    assert_eq!(draft.caret_cell(), (0, 4));
}

#[test]
fn adds_a_line() {
    let draft = typed("one\ntwo");

    assert_eq!(draft.rows().len(), 2);
    assert_eq!(draft.caret_cell(), (1, 3));
}

#[test]
fn cleans_pasted_text() {
    let draft = typed("a\r\nb\rc\td\u{7}e");

    assert_eq!(draft.text(), "a\nb\nc    de");
}

#[test]
fn ignores_text_with_nothing_to_keep() {
    let draft = typed("\u{1b}\u{7}");

    assert_eq!(draft, Draft::default());
}

#[test]
fn caps_the_draft_at_a_million_characters() {
    let mut draft = typed(&"a".repeat(LONGEST - 2));

    draft.apply(Edit::Insert("bcdef".to_owned()));

    assert_eq!(draft.text().chars().count(), LONGEST);
    assert!(draft.text().ends_with("abc"));
}

#[test]
fn erases_before_and_after_the_caret() {
    let mut draft = typed("abcd");
    after(
        &mut draft,
        [Edit::Move(Motion::Left), Edit::Move(Motion::Left)],
    );

    after(
        &mut draft,
        [Edit::Erase(Motion::Left), Edit::Erase(Motion::Right)],
    );

    assert_eq!(draft.text(), "ad");
    assert_eq!(draft.caret_cell(), (0, 1));
}

#[test]
fn erases_a_word_and_back_to_the_start_of_the_row() {
    let mut draft = typed("one two three");

    draft.apply(Edit::Erase(Motion::WordLeft));
    assert_eq!(draft.text(), "one two ");

    draft.apply(Edit::Erase(Motion::RowStart));
    assert_eq!(draft.text(), "");
}

#[test]
fn erasing_to_the_row_start_from_a_row_start_erases_the_character_before() {
    let mut draft = typed(&"x".repeat(70));
    draft.place(1, 0);

    draft.apply(Edit::Erase(Motion::RowStart));

    assert_eq!(draft.text().len(), 69);
    assert_eq!(draft.caret_cell(), (0, 61));
}

#[test]
fn types_after_the_last_character_of_a_cut_row() {
    let mut draft = typed(&"x".repeat(70));
    draft.apply(Edit::Move(Motion::Start));
    draft.apply(Edit::Move(Motion::RowEnd));

    draft.apply(Edit::Insert("y".to_owned()));

    assert_eq!(draft.text().find('y'), Some(TEXT_WIDTH));
}

#[test]
fn erasing_at_the_start_changes_nothing() {
    let mut draft = typed("abc");
    draft.apply(Edit::Move(Motion::Start));
    let before = draft.clone();

    draft.apply(Edit::Erase(Motion::Left));

    assert_eq!(draft, before);
}

#[test]
fn keeps_the_goal_column_across_short_rows() {
    let mut draft = typed("abcdefgh\nab\nabcdefgh");
    draft.apply(Edit::Move(Motion::Up));
    draft.apply(Edit::Move(Motion::Up));

    assert_eq!(draft.caret_cell(), (0, 8));

    draft.apply(Edit::Move(Motion::Left));
    draft.apply(Edit::Move(Motion::Down));
    assert_eq!(draft.caret_cell(), (1, 2));
    draft.apply(Edit::Move(Motion::Down));
    assert_eq!(draft.caret_cell(), (2, 7));
}

#[test]
fn forgets_the_goal_column_after_typing() {
    let mut draft = typed("abcdefgh\nab\nabcdefgh");
    after(
        &mut draft,
        [
            Edit::Move(Motion::Up),
            Edit::Insert("x".to_owned()),
            Edit::Move(Motion::Down),
        ],
    );

    assert_eq!(draft.caret_cell(), (2, 3));
}

#[test]
fn places_the_caret_where_it_is_clicked() {
    let mut draft = typed("hello\nworld");

    draft.place(0, 2);
    assert_eq!(draft.caret_cell(), (0, 2));

    draft.place(5, 99);
    assert_eq!(draft.caret_cell(), (1, 5));
}

#[test]
fn takes_the_text_to_send_and_starts_over() {
    let mut draft = typed("\n \n  indented\nlast  \n\n");

    assert_eq!(draft.take(), Some("  indented\nlast".to_owned()));
    assert_eq!(draft, Draft::default());
}

#[test]
fn sends_nothing_when_only_blanks_were_typed() {
    let mut draft = typed(" \n\n ");

    assert_eq!(draft.take(), None);
    assert_eq!(draft.text(), " \n\n ");
}

proptest! {
    #[test]
    fn any_edit_keeps_the_caret_on_a_character(
        texts in prop::collection::vec("[a-c é\n\t\r]{0,12}", 1..6),
        motions in prop::collection::vec(0_usize..10, 0..12),
        erase in prop::collection::vec(any::<bool>(), 12),
    ) {
        const MOTIONS: [Motion; 10] = [
            Motion::Left, Motion::Right, Motion::WordLeft, Motion::WordRight, Motion::RowStart,
            Motion::RowEnd, Motion::Up, Motion::Down, Motion::Start, Motion::End,
        ];
        let mut draft = Draft::default();
        for text in texts {
            draft.apply(Edit::Insert(text));
            for (&motion, &erases) in motions.iter().zip(&erase) {
                let edit = if erases { Edit::Erase(MOTIONS[motion]) } else { Edit::Move(MOTIONS[motion]) };
                draft.apply(edit);

                prop_assert!(draft.text().is_char_boundary(draft.caret.offset));
                prop_assert!(draft.caret.offset <= draft.text().len());
                prop_assert!(!draft.text().contains(['\r', '\t']));
                let (row, column) = draft.caret_cell();
                prop_assert!(row < draft.rows().len());
                prop_assert_eq!(draft.cursor().spot_at(row, column).offset, draft.caret.offset);
            }
        }
    }
}
