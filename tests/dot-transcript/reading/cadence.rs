use std::time::{Duration, Instant};

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn earns_nothing_on_the_first_call_nor_with_nothing_waiting() {
    let start = Instant::now();
    let mut cadence = Cadence::default();

    assert_eq!(cadence.allowance(40, start), 0);
    assert_eq!(cadence.allowance(0, after(start, 100)), 0);
    assert_eq!(cadence.allowance(40, after(start, 100)), 0);
}

#[test]
fn shows_eighty_characters_a_second_when_little_waits() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    cadence.allowance(30, start);

    assert_eq!(cadence.allowance(30, after(start, 50)), 4);
    assert_eq!(cadence.allowance(30, after(start, 100)), 8);
    cadence.spend(8);
    assert_eq!(cadence.allowance(30, after(start, 150)), 4);
}

#[test]
fn catches_up_within_half_a_second_when_much_waits() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    cadence.allowance(1_000, start);

    assert_eq!(cadence.allowance(1_000, after(start, 50)), 100);
}

#[test]
fn earns_a_tenth_of_a_second_at_most_and_never_more_than_waits() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    cadence.allowance(30, start);

    assert_eq!(cadence.allowance(30, after(start, 5_000)), 8);
    assert_eq!(cadence.allowance(5, after(start, 5_100)), 5);
}

#[test]
fn rests_from_nothing_after_being_held_back() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    cadence.allowance(30, start);
    cadence.allowance(30, after(start, 100));

    cadence.rest();

    assert_eq!(cadence.allowance(30, after(start, 200)), 0);
    assert_eq!(cadence.allowance(30, after(start, 250)), 4);
}

#[test]
fn spends_no_more_than_it_has() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    cadence.allowance(30, start);
    cadence.allowance(30, after(start, 50));

    cadence.spend(100);

    assert_eq!(cadence.allowance(30, after(start, 50)), 0);
}

#[test]
fn shows_whole_words_with_the_spaces_before_them() {
    let text = "La largeur de la colonne";

    assert_eq!(words_within(text, 0), 0);
    assert_eq!(words_within(text, 1), 0);
    assert_eq!(words_within(text, 2), 2);
    assert_eq!(words_within(text, 10), 10);
    assert_eq!(words_within(text, 11), 10);
    assert_eq!(words_within(text, 13), 13);
    assert_eq!(words_within(text, 100), text.len());
    assert_eq!(words_within("\n\nSuite", 7), "\n\nSuite".len());
    assert_eq!(words_within("  ", 1), 0);
    assert_eq!(words_within("  ", 2), 2);
}

#[test]
fn counts_characters_not_bytes() {
    let text = "déjà été";

    assert_eq!(words_within(text, 4), "déjà".len());
    assert_eq!(words_within(text, 8), text.len());
}

#[test]
fn shows_a_long_word_a_part_at_a_time() {
    let hash = "a".repeat(40);
    let path = format!(" {}", "é".repeat(20));

    assert_eq!(words_within(&hash, 10), 10);
    assert_eq!(words_within(&format!("ok {hash}"), 12), 2 + 1 + 9);
    assert_eq!(words_within(&path, 5), 1 + 4 * "é".len());
    assert_eq!(words_within(&"b".repeat(16), 10), 0);
    assert_eq!(words_within(&format!("  {}", "b".repeat(15)), 10), 0);
    assert_eq!(words_within(&format!("  {}", "b".repeat(17)), 10), 10);
}
