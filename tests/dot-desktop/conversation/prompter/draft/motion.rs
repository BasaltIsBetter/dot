use super::*;
use dot_transcript::wrap;

const WIDTH: usize = 10;

fn target(text: &str, caret: usize, motion: Motion) -> usize {
    target_with_goal(text, caret, motion, column_of(text, caret))
}

fn target_with_goal(text: &str, caret: usize, motion: Motion, goal: usize) -> usize {
    spot_with_goal(text, Spot::at(caret), motion, goal).offset
}

fn spot_with_goal(text: &str, caret: Spot, motion: Motion, goal: usize) -> Spot {
    let rows = wrap(text, WIDTH);
    Cursor {
        text,
        rows: &rows,
        caret,
    }
    .target(motion, goal)
}

fn column_of(text: &str, caret: usize) -> usize {
    cell_of(text, Spot::at(caret)).1
}

fn cell(text: &str, caret: usize) -> (usize, usize) {
    cell_of(text, Spot::at(caret))
}

fn cell_of(text: &str, caret: Spot) -> (usize, usize) {
    let rows = wrap(text, WIDTH);
    let cursor = Cursor {
        text,
        rows: &rows,
        caret,
    };
    (cursor.row(), cursor.column())
}

#[test]
fn steps_one_character_at_a_time_even_across_accents() {
    let text = "aé b";

    assert_eq!(target(text, 1, Motion::Right), 3);
    assert_eq!(target(text, 3, Motion::Left), 1);
    assert_eq!(target(text, 0, Motion::Left), 0);
    assert_eq!(target(text, text.len(), Motion::Right), text.len());
}

#[test]
fn jumps_over_words_like_macos() {
    let text = "let foo_bar = baz(1);";

    assert_eq!(target(text, text.len(), Motion::WordLeft), 18);
    assert_eq!(target(text, 18, Motion::WordLeft), 14);
    assert_eq!(target(text, 14, Motion::WordLeft), 4);
    assert_eq!(target(text, 3, Motion::WordLeft), 0);
    assert_eq!(target(text, 0, Motion::WordLeft), 0);
    assert_eq!(target(text, 0, Motion::WordRight), 3);
    assert_eq!(target(text, 3, Motion::WordRight), 11);
    assert_eq!(target(text, 18, Motion::WordRight), 19);
    assert_eq!(target(text, 19, Motion::WordRight), text.len());
}

#[test]
fn finds_the_row_and_column_of_the_caret() {
    let text = "hello world\nnext";

    assert_eq!(cell(text, 0), (0, 0));
    assert_eq!(cell(text, 5), (0, 5));
    assert_eq!(cell(text, 6), (1, 0));
    assert_eq!(cell(text, 11), (1, 5));
    assert_eq!(cell(text, 12), (2, 0));
    assert_eq!(cell(text, text.len()), (2, 4));
}

#[test]
fn goes_to_the_ends_of_the_row() {
    let text = "hello world\nnext";

    assert_eq!(target(text, 8, Motion::RowStart), 6);
    assert_eq!(target(text, 8, Motion::RowEnd), 11);
    assert_eq!(
        spot_with_goal(
            text,
            Spot {
                offset: 6,
                upstream: true
            },
            Motion::RowStart,
            0
        ),
        Spot::at(0)
    );
    assert_eq!(target(text, 13, Motion::RowEnd), text.len());
}

#[test]
fn ends_a_wrapped_row_after_its_last_character_and_stays_on_it() {
    let words = "hello world";
    let long_word = "x".repeat(25);

    let after_space = spot_with_goal(words, Spot::at(1), Motion::RowEnd, 0);
    let after_cut = spot_with_goal(&long_word, Spot::at(1), Motion::RowEnd, 0);

    assert_eq!(
        after_space,
        Spot {
            offset: 6,
            upstream: true
        }
    );
    assert_eq!(cell_of(words, after_space), (0, 6));
    assert_eq!(
        after_cut,
        Spot {
            offset: 10,
            upstream: true
        }
    );
    assert_eq!(cell_of(&long_word, after_cut), (0, 10));
    assert_eq!(cell(&long_word, 10), (1, 0));
}

#[test]
fn ends_the_last_row_without_affinity() {
    assert_eq!(
        spot_with_goal("hello", Spot::at(1), Motion::RowEnd, 0),
        Spot::at(5)
    );
}

#[test]
fn moving_down_past_a_cut_row_end_stays_on_that_row() {
    let text = "x".repeat(25);

    let down = spot_with_goal(&text, Spot::at(3), Motion::Down, 99);

    assert_eq!(cell_of(&text, down), (1, 10));
    assert!(down.upstream);
}

#[test]
fn ignores_affinity_away_from_a_row_end() {
    let caret = Spot {
        offset: 3,
        upstream: true,
    };

    assert_eq!(cell_of("hello world", caret), (0, 3));
}

#[test]
fn moves_up_and_down_keeping_the_goal_column() {
    let text = "abcdefgh\nab\nabcdefgh";

    assert_eq!(target_with_goal(text, 7, Motion::Down, 7), 11);
    assert_eq!(target_with_goal(text, 11, Motion::Down, 7), 19);
    assert_eq!(target_with_goal(text, 19, Motion::Up, 7), 11);
}

#[test]
fn goes_to_the_ends_of_the_text_past_the_first_and_last_rows() {
    let text = "abc\ndef";

    assert_eq!(target(text, 2, Motion::Up), 0);
    assert_eq!(target(text, 5, Motion::Down), text.len());
}

#[test]
fn places_a_spot_by_row_and_column_inside_the_text() {
    let text = "hello world\nnext";
    let rows = wrap(text, WIDTH);
    let cursor = Cursor {
        text,
        rows: &rows,
        caret: Spot::default(),
    };

    assert_eq!(cursor.spot_at(0, 2), Spot::at(2));
    assert_eq!(
        cursor.spot_at(0, 99),
        Spot {
            offset: 6,
            upstream: true
        }
    );
    assert_eq!(cursor.spot_at(1, 99), Spot::at(11));
    assert_eq!(cursor.spot_at(99, 1), Spot::at(13));
}
