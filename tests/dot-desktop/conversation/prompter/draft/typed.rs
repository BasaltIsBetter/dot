use std::time::{Duration, Instant};

use super::*;

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn typed_text_arrives_orange_and_cools_in_four_hundred_fifty_milliseconds() {
    let start = Instant::now();
    let mut typed = Typed::default();

    typed.inserted(0, 3, start);

    assert_eq!(typed.heat_at(0, start), u8::MAX);
    let cooling = typed.heat_at(2, after(start, 200));
    assert!(cooling > 0 && cooling < u8::MAX, "{cooling}");
    assert_eq!(typed.heat_at(1, after(start, 450)), 0);
    assert_eq!(typed.heat_at(3, start), 0);
    assert!(typed.is_cooling(after(start, 449)));
    assert!(!typed.is_cooling(after(start, 450)));
}

#[test]
fn text_typed_before_moves_along_when_text_is_inserted_in_front() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 2, start);

    typed.inserted(0, 3, after(start, 100));

    let runs = typed.runs(0..5, after(start, 100));
    assert_eq!(runs[0], (0..3, u8::MAX));
    assert_eq!(runs[1].0, 3..5);
    assert!(runs[1].1 > 0 && runs[1].1 < u8::MAX, "{runs:?}");
}

#[test]
fn an_insertion_inside_earlier_text_stretches_it() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 4, start);

    typed.inserted(2, 2, after(start, 100));

    let runs = typed.runs(0..6, after(start, 100));
    let edges: Vec<_> = runs.iter().map(|(piece, _)| piece.clone()).collect();
    assert_eq!(edges, [0..2, 2..4, 4..6]);
    assert_eq!(runs[0].1, runs[2].1);
    assert_eq!(runs[1].1, u8::MAX);
}

#[test]
fn erased_text_takes_its_heat_away() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 3, start);
    typed.inserted(3, 3, after(start, 100));

    typed.erased(&(1..4), after(start, 100));

    let runs = typed.runs(0..3, after(start, 100));
    let edges: Vec<_> = runs.iter().map(|(piece, _)| piece.clone()).collect();
    assert_eq!(edges, [0..1, 1..3]);
    assert_eq!(runs[1].1, u8::MAX);
}

#[test]
fn erasing_all_of_an_arrival_forgets_it() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 3, start);

    typed.erased(&(0..3), start);

    assert!(!typed.is_cooling(start));
}

#[test]
fn forgets_cold_arrivals_at_the_next_change() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 3, start);

    typed.inserted(3, 1, after(start, 1_000));

    assert_eq!(
        typed.runs(0..4, after(start, 1_000)),
        vec![(0..3, 0), (3..4, u8::MAX)]
    );
}

#[test]
fn a_row_without_typing_is_one_cold_run_and_an_empty_row_none() {
    let typed = Typed::default();

    assert_eq!(typed.runs(2..9, Instant::now()), vec![(2..9, 0)]);
    assert_eq!(typed.runs(4..4, Instant::now()), Vec::new());
}

#[test]
fn text_moved_along_keeps_its_length_and_nothing_after_it_heats() {
    let start = Instant::now();
    let mut in_front = Typed::default();
    in_front.inserted(0, 2, start);
    let mut inside = Typed::default();
    inside.inserted(0, 4, start);

    in_front.inserted(0, 3, after(start, 100));
    inside.inserted(2, 2, after(start, 100));

    let moved = in_front.runs(0..6, after(start, 100));
    let stretched = inside.runs(0..8, after(start, 100));
    assert_eq!(moved.last().unwrap(), &(5..6, 0));
    assert_eq!(stretched.last().unwrap(), &(6..8, 0));
}

#[test]
fn joins_neighbours_of_the_same_heat_into_one_run() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 2, start);
    typed.inserted(2, 2, start);

    assert_eq!(typed.runs(0..4, start), [(0..4, u8::MAX)]);
}

#[test]
fn forgets_what_cooled_once_something_else_is_typed() {
    let start = Instant::now();
    let mut typed = Typed::default();
    typed.inserted(0, 1, start);
    typed.inserted(1, 1, after(start, 449));

    let warm = typed.fresh.len();
    typed.inserted(2, 1, after(start, 450));

    assert_eq!(warm, 2);
    assert_eq!(typed.fresh.len(), 2);
}
