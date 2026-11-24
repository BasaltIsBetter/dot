use std::time::Instant;

use proptest::prelude::*;

use super::*;
use crate::conversation::stage::text_width;

fn typed(text: &str) -> Draft {
    let mut draft = Draft::default();
    draft.apply(Edit::Insert(text.to_owned()), Instant::now());
    draft
}

fn after(draft: &mut Draft, edits: impl IntoIterator<Item = Edit>) {
    for edit in edits {
        draft.apply(edit, Instant::now());
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

    assert_eq!(draft.text(), "a\nb\nc\tde");
}

#[test]
fn keeps_a_pasted_tab_and_counts_it_four_columns() {
    let mut draft = typed("\tx");

    assert_eq!(draft.caret_cell(), (0, 5));
    draft.apply(Edit::Move(Motion::Left), Instant::now());
    draft.apply(Edit::Move(Motion::Left), Instant::now());
    assert_eq!(draft.caret_cell(), (0, 0));
    draft.apply(Edit::Move(Motion::Right), Instant::now());
    assert_eq!(draft.caret_cell(), (0, 4));
}

#[test]
fn sends_tabs_untouched() {
    let mut draft = typed("all:\n\tcargo build");

    assert_eq!(draft.take(), Some("all:\n\tcargo build".to_owned()));
}

#[test]
fn ignores_text_with_nothing_to_keep() {
    let draft = typed("\u{1b}\u{7}");

    assert_eq!(draft, Draft::default());
}

#[test]
fn caps_the_draft_at_a_million_characters() {
    let mut draft = typed(&"a".repeat(LONGEST - 2));

    draft.apply(Edit::Insert("bcdef".to_owned()), Instant::now());

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

    draft.apply(Edit::Erase(Motion::WordLeft), Instant::now());
    assert_eq!(draft.text(), "one two ");

    draft.apply(Edit::Erase(Motion::RowStart), Instant::now());
    assert_eq!(draft.text(), "");
}

#[test]
fn erasing_to_the_row_start_from_a_row_start_erases_the_character_before() {
    let width = TextWidth::default().columns();
    let mut draft = typed(&"x".repeat(width + 8));
    draft.place(1, 0);

    draft.apply(Edit::Erase(Motion::RowStart), Instant::now());

    assert_eq!(draft.text().len(), width + 7);
    assert_eq!(draft.caret_cell(), (0, width - 1));
}

#[test]
fn types_after_the_last_character_of_a_cut_row() {
    let width = TextWidth::default().columns();
    let mut draft = typed(&"x".repeat(width + 8));
    draft.apply(Edit::Move(Motion::Up), Instant::now());
    draft.apply(Edit::Move(Motion::RowEnd), Instant::now());

    draft.apply(Edit::Insert("y".to_owned()), Instant::now());

    assert_eq!(draft.text().find('y'), Some(width));
}

#[test]
fn erasing_at_the_start_changes_nothing() {
    let mut draft = typed("abc");
    draft.apply(Edit::Move(Motion::Up), Instant::now());
    let before = draft.clone();

    draft.apply(Edit::Erase(Motion::Left), Instant::now());

    assert_eq!(draft, before);
}

#[test]
fn keeps_the_goal_column_across_short_rows() {
    let mut draft = typed("abcdefgh\nab\nabcdefgh");
    draft.apply(Edit::Move(Motion::Up), Instant::now());
    draft.apply(Edit::Move(Motion::Up), Instant::now());

    assert_eq!(draft.caret_cell(), (0, 8));

    draft.apply(Edit::Move(Motion::Left), Instant::now());
    draft.apply(Edit::Move(Motion::Down), Instant::now());
    assert_eq!(draft.caret_cell(), (1, 2));
    draft.apply(Edit::Move(Motion::Down), Instant::now());
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
        motions in prop::collection::vec(0_usize..8, 0..12),
        erase in prop::collection::vec(any::<bool>(), 12),
    ) {
        const MOTIONS: [Motion; 8] = [
            Motion::Left, Motion::Right, Motion::WordLeft, Motion::WordRight, Motion::RowStart,
            Motion::RowEnd, Motion::Up, Motion::Down,
        ];
        let mut draft = Draft::default();
        for text in texts {
            draft.apply(Edit::Insert(text), Instant::now());
            for (&motion, &erases) in motions.iter().zip(&erase) {
                let edit = if erases { Edit::Erase(MOTIONS[motion]) } else { Edit::Move(MOTIONS[motion]) };
                draft.apply(edit, Instant::now());

                prop_assert!(draft.text().is_char_boundary(draft.caret.offset));
                prop_assert!(draft.caret.offset <= draft.text().len());
                prop_assert!(!draft.text().contains('\r'));
                let (row, column) = draft.caret_cell();
                prop_assert!(row < draft.rows().len());
                prop_assert_eq!(draft.cursor().spot_at(row, column).offset, draft.caret.offset);
            }
        }
    }
}

fn narrowest() -> TextWidth {
    text_width(&crate::canvas::Layout::reflowed(
        winit::dpi::PhysicalSize::new(0, 0),
        1.0,
    ))
}

#[test]
fn rewraps_its_text_to_a_new_width_and_keeps_the_caret_on_its_character() {
    let mut draft = typed("one two three four five six seven eight");
    draft.apply(Edit::Move(Motion::WordLeft), Instant::now());

    draft.rewrap_to(narrowest());

    assert_eq!(draft.width(), narrowest());
    assert_eq!(draft.rows(), vec![0..22, 22..39]);
    assert_eq!(draft.caret_cell(), (1, 12));
    draft.apply(Edit::Insert("x".to_owned()), Instant::now());
    assert_eq!(draft.text(), "one two three four five six seven xeight");
}

#[test]
fn keeps_its_width_once_sent_or_cleared() {
    let mut draft = typed("hello");
    draft.rewrap_to(narrowest());

    draft.take();
    let sent = draft.width();
    after(&mut draft, [Edit::Insert("again".to_owned())]);
    draft.clear();

    assert_eq!(sent, narrowest());
    assert_eq!(draft.width(), narrowest());
    assert_eq!(draft.text(), "");
    assert_eq!(draft.rows(), vec![0..0]);
}

#[test]
fn keeps_count_of_its_characters_through_every_edit() {
    let now = Instant::now();
    let mut draft = typed("héllo wörld");
    draft.apply(Edit::Erase(Motion::Left), now);
    draft.replace(0..3, "ÉÉÉ", now);

    assert_eq!(draft.characters, draft.text().chars().count());
    assert_eq!(draft.room(), LONGEST - draft.text().chars().count());

    draft.clear();
    assert_eq!(draft.room(), LONGEST);
}

#[test]
fn a_draft_made_from_a_text_has_the_room_it_is_given() {
    let draft = Draft::from_text("ab".to_owned(), TextWidth::default(), 5);

    assert_eq!(draft.room(), 3);
}

#[test]
fn clearing_keeps_the_room_it_was_given() {
    let mut draft = Draft::from_text("ab".to_owned(), TextWidth::default(), 5);

    draft.clear();

    assert_eq!(draft.room(), 5);
}

proptest! {
    #[test]
    fn its_count_of_characters_never_drifts(
        typed_texts in proptest::collection::vec("[a-zé😀\n ]{0,6}", 1..8),
        erased in 0usize..6,
    ) {
        let now = Instant::now();
        let mut draft = Draft::default();
        for text in &typed_texts {
            draft.apply(Edit::Insert(text.clone()), now);
        }
        for _ in 0..erased {
            draft.apply(Edit::Erase(Motion::Left), now);
        }

        prop_assert_eq!(draft.characters, draft.text().chars().count());
    }
}
