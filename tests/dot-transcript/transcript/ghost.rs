use std::time::Duration;

use proptest::prelude::*;

use super::*;
use crate::line::TextWidth;
use crate::transcript::entry::Kind;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn question(texts: &[(Kind, &str)], now: Instant) -> Vec<Entry> {
    texts
        .iter()
        .map(|(kind, text)| Entry::new(*kind, text, now, TextWidth::default()))
        .collect()
}

fn ghost_of(texts: &[(Kind, &str)], now: Instant) -> Ghost {
    let entries = question(texts, now);
    Ghost::new(0..entries.len(), &entries)
}

#[test]
fn splits_a_row_into_words_with_their_columns() {
    let found: Vec<_> = words("  hé you  there ").collect();

    assert_eq!(found, vec![(2, 2..5), (5, 6..9), (10, 11..16)]);
    assert_eq!(words("").count(), 0);
    assert_eq!(words("   ").count(), 0);
}

#[test]
fn waits_until_the_reply_reaches_a_word() {
    let start = Instant::now();
    let ghost = ghost_of(&[(Kind::Prompt, "how are you")], start);

    assert_eq!(ghost.fate((0, 0), start), Fate::Waiting);
    assert_eq!(ghost.fate((0, 8), start), Fate::Waiting);
    assert!(!ghost.is_complete());
    assert!(!ghost.is_rolling(start));
}

#[test]
fn a_reached_word_leaves_for_three_hundred_milliseconds_then_is_gone() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how are you")], start);

    ghost.advance((0, 5), start);

    let Fate::Leaving(halfway) = ghost.fate((0, 4), after(start, 150)) else {
        panic!("the word should be leaving");
    };
    assert!((halfway - 0.684_643_187_4).abs() < 1e-6);
    assert!(matches!(ghost.fate((0, 0), start), Fate::Leaving(eased) if eased.abs() < 1e-6));
    assert_eq!(ghost.fate((0, 4), after(start, 300)), Fate::Gone);
    assert_eq!(ghost.fate((0, 8), after(start, 300)), Fate::Waiting);
    assert!(ghost.is_rolling(after(start, 299)));
    assert!(!ghost.is_rolling(after(start, 300)));
}

#[test]
fn a_word_starting_at_the_cursor_still_waits() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how are you")], start);

    ghost.advance((0, 4), start);

    assert_eq!(ghost.fate((0, 4), start), Fate::Waiting);
    assert_ne!(ghost.fate((0, 0), start), Fate::Waiting);
}

#[test]
fn each_word_leaves_when_the_reply_first_passes_it() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how are you")], start);

    ghost.advance((0, 2), start);
    ghost.advance((0, 6), after(start, 100));

    assert_eq!(ghost.fate((0, 0), after(start, 300)), Fate::Gone);
    assert!(matches!(
        ghost.fate((0, 4), after(start, 300)),
        Fate::Leaving(_)
    ));
    assert_eq!(ghost.fate((0, 4), after(start, 400)), Fate::Gone);
}

#[test]
fn a_later_row_of_the_reply_passes_the_rest_of_the_row_above() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how are you\ntoday")], start);

    ghost.advance((1, 0), start);

    assert_ne!(ghost.fate((0, 8), start), Fate::Waiting);
    assert_eq!(ghost.fate((1, 0), start), Fate::Waiting);
}

#[test]
fn is_complete_once_the_reply_passes_the_last_word() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how are you\ntoday")], start);

    ghost.advance((1, 0), start);
    let before = ghost.is_complete();
    ghost.advance((1, 1), start);
    ghost.advance((1, 9), start);

    assert!(!before);
    assert!(ghost.is_complete());
    assert!(ghost.rolls(start));
    assert!(!ghost.rolls(after(start, 1)));
}

#[test]
fn every_arrival_rolls_in_until_the_ghost_is_complete() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how")], start);

    let before = ghost.rolls(after(start, 5_000));
    ghost.advance((0, 2), after(start, 100));

    assert!(before);
    assert!(ghost.rolls(after(start, 100)));
    assert!(!ghost.rolls(after(start, 101)));
}

#[test]
fn hides_the_question_until_complete() {
    let start = Instant::now();
    let entries = question(
        &[(Kind::Prompt, "look"), (Kind::Attachment, "image")],
        start,
    );
    let mut ghost = Ghost::new(0..2, &entries);

    let hidden = [ghost.hides(0), ghost.hides(1), ghost.hides(2)];
    ghost.finish(start);

    assert_eq!(hidden, [true, true, false]);
    assert!(!ghost.hides(0));
    assert_eq!(ghost.question(), 0..2);
}

#[test]
fn finishing_sends_every_word_left_away_at_once() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how are you\ntoday")], start);
    ghost.advance((0, 2), start);

    ghost.finish(after(start, 50));

    assert!(ghost.is_complete());
    assert!(matches!(
        ghost.fate((1, 0), after(start, 50)),
        Fate::Leaving(_)
    ));
    assert!(ghost.is_rolling(after(start, 349)));
    assert!(ghost.is_settled(after(start, 350)));
    assert!(ghost.rolls(after(start, 50)));
    assert!(!ghost.rolls(after(start, 51)));
}

#[test]
fn finishing_twice_keeps_the_first_departure() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "how")], start);
    ghost.finish(start);

    ghost.finish(after(start, 200));
    ghost.advance((5, 0), after(start, 200));

    assert!(!ghost.is_rolling(after(start, 300)));
}

#[test]
fn a_rebased_ghost_lets_reached_words_go_without_moving() {
    let start = Instant::now();
    let entries = question(&[(Kind::Prompt, "how are you")], start);
    let mut ghost = Ghost::new(0..1, &entries);
    ghost.advance((0, 5), start);

    ghost.rebase(&entries, (0, 5), start);

    assert_eq!(ghost.fate((0, 4), start), Fate::Gone);
    assert_eq!(ghost.fate((0, 8), start), Fate::Waiting);
    assert!(!ghost.is_rolling(start));
    assert!(!ghost.is_complete());
}

#[test]
fn rebasing_past_the_last_word_completes_the_ghost() {
    let start = Instant::now();
    let entries = question(&[(Kind::Prompt, "how are you")], start);
    let mut ghost = Ghost::new(0..1, &entries);

    ghost.rebase(&entries, (1, 0), start);

    assert!(ghost.is_complete());
    assert!(ghost.rolls(start));
    assert!(!ghost.rolls(after(start, 1)));
}

#[test]
fn a_complete_ghost_stays_complete_when_rebased() {
    let start = Instant::now();
    let entries = question(&[(Kind::Prompt, "how are you")], start);
    let mut ghost = Ghost::new(0..1, &entries);
    ghost.finish(start);

    ghost.rebase(&entries, (0, 0), start);

    assert!(ghost.is_complete());
    assert_eq!(ghost.fate((0, 8), start), Fate::Gone);
    assert!(ghost.rolls(start));
}

#[test]
fn the_last_word_may_sit_on_an_image_row() {
    let start = Instant::now();
    let mut ghost = ghost_of(
        &[
            (Kind::Prompt, "look"),
            (Kind::Attachment, "image  a.png  2 KB"),
        ],
        start,
    );

    ghost.advance((1, 16), start);
    let before = ghost.is_complete();
    ghost.advance((1, 17), start);

    assert!(!before);
    assert!(ghost.is_complete());
}

#[test]
fn an_empty_prompt_is_complete_once_anything_is_written() {
    let start = Instant::now();
    let mut ghost = ghost_of(&[(Kind::Prompt, "")], start);

    ghost.advance((0, 0), start);
    let before = ghost.is_complete();
    ghost.advance((0, 1), start);

    assert!(!before);
    assert!(ghost.is_complete());
}

proptest! {
    #[test]
    fn reached_words_are_always_a_prefix_of_the_question(
        cursors in prop::collection::vec((0_usize..4, 0_usize..40), 1..8),
        cells in prop::collection::vec((0_usize..4, 0_usize..40), 2..20),
    ) {
        let start = Instant::now();
        let mut ghost = ghost_of(&[(Kind::Prompt, "a b c d e f g h i j k l m n o p q r s t")], start);
        let mut cursors = cursors;
        cursors.sort_unstable();
        for (step, cursor) in cursors.into_iter().enumerate() {
            ghost.advance(cursor, after(start, 10 * u64::try_from(step).unwrap()));
        }
        let mut cells = cells;
        cells.sort_unstable();

        let waiting: Vec<bool> = cells
            .iter()
            .map(|&cell| ghost.fate(cell, after(start, 100)) == Fate::Waiting)
            .collect();

        prop_assert!(waiting.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}

#[test]
fn rebasing_onto_the_last_word_leaves_it_waiting() {
    let start = Instant::now();
    let entries = question(&[(Kind::Prompt, "how are you")], start);
    let mut ghost = Ghost::new(0..1, &entries);

    ghost.rebase(&entries, (0, 8), start);

    assert!(!ghost.is_complete());
    assert_eq!(ghost.fate((0, 8), start), Fate::Waiting);
}
