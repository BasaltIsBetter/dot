use proptest::prelude::*;

use super::*;
use crate::line::TextWidth;
use crate::transcript::tool::{Glyph, Tool};

fn at(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

fn width() -> TextWidth {
    TextWidth::default()
}

fn pieces(entry: &Entry, row: Range<usize>, now: Instant) -> Vec<(Range<usize>, u8)> {
    let mut pieces = Vec::new();
    entry.each_piece(row, now, |piece, heat| pieces.push((piece, heat)));
    pieces
}

fn after(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

#[test]
fn arrives_fully_orange() {
    assert_eq!(heat(Duration::ZERO), u8::MAX);
}

#[test]
fn cools_along_the_css_ease_out_curve() {
    assert_eq!(heat(after(100)), 221);
    assert_eq!(heat(after(300)), 159);
    assert_eq!(heat(after(600)), 80);
    assert_eq!(heat(after(900)), 24);
    assert_eq!(heat(after(1_100)), 3);
}

#[test]
fn settles_to_its_ink_after_one_point_two_seconds() {
    assert_eq!(heat(after(1_200)), 0);
    assert_eq!(heat(after(60_000)), 0);
}

#[test]
fn stays_fresh_until_it_has_settled() {
    assert!(is_fresh(Duration::ZERO));
    assert!(is_fresh(after(1_199)));
    assert!(!is_fresh(after(1_200)));
}

#[test]
fn splits_a_row_where_its_pieces_arrived() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "abc", start, width());
    entry.extend("def", at(start, 600), width());

    let pieces = pieces(&entry, 0..6, at(start, 600));

    assert_eq!(pieces, vec![(0..3, 80), (3..6, u8::MAX)]);
}

#[test]
fn keeps_a_settled_row_in_one_piece() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "abc", start, width());
    entry.extend("def", at(start, 100), width());

    let pieces = pieces(&entry, 0..6, at(start, 5_000));

    assert_eq!(pieces, vec![(0..6, 0)]);
}

#[test]
fn heats_a_row_after_the_last_piece_that_started_before_it() {
    let start = Instant::now();
    let long = "word ".repeat(20);
    let entry = Entry::new(Kind::Answer, &long, start, width());
    let second_row = entry.row(1);

    let pieces = pieces(&entry, second_row.clone(), start);

    assert_eq!(pieces, vec![(second_row, u8::MAX)]);
}

#[test]
fn cools_over_whatever_time_it_is_given() {
    let typing = Duration::from_millis(450);

    assert_eq!(heat_after(Duration::ZERO, typing), u8::MAX);
    assert_eq!(heat_after(after(225), typing), heat(after(600)));
    assert!(heat_after(after(400), typing) > 0);
    assert_eq!(heat_after(after(450), typing), 0);
}

#[test]
fn a_tool_stays_orange_while_it_runs_then_cools_from_its_end() {
    let start = Instant::now();
    let tool = Tool {
        glyph: Glyph::Read,
        label: "layout.rs".to_owned(),
    };
    let mut entry = Entry::tool(tool, start);

    assert_eq!(entry.heat(None, at(start, 5_000)), u8::MAX);

    entry.ended = Some(at(start, 5_000));

    assert_eq!(entry.heat(None, at(start, 5_000)), u8::MAX);
    assert_eq!(entry.heat(None, at(start, 5_600)), 80);
    assert_eq!(entry.heat(None, at(start, 6_200)), 0);
}

#[test]
fn remembers_when_the_last_piece_arrived() {
    let start = Instant::now();
    let mut entry = Entry::empty(Kind::Answer, width());

    assert_eq!(entry.last_arrival(), None);

    entry.extend("a", start, width());
    entry.extend("b", at(start, 40), width());

    assert_eq!(entry.last_arrival(), Some(at(start, 40)));
}

proptest! {
    #[test]
    fn only_ever_cools(earlier in 0_u64..2_000, later_by in 0_u64..2_000) {
        prop_assert!(heat(after(earlier)) >= heat(after(earlier + later_by)));
    }

    #[test]
    fn covers_each_row_with_its_pieces(
        arriving in proptest::collection::vec("[a-z ]{1,30}", 1..10),
        elapsed in 0_u64..2_000,
    ) {
        let start = Instant::now();
        let mut entry = Entry::empty(Kind::Answer, width());
        for (index, piece) in arriving.iter().enumerate() {
            entry.extend(piece, at(start, 100 * u64::try_from(index).unwrap()), width());
        }

        for row in entry.rows().collect::<Vec<_>>() {
            let covered = pieces(&entry, row.clone(), at(start, elapsed));
            prop_assert_eq!(covered.first().map(|(range, _)| range.start), Some(row.start));
            prop_assert_eq!(covered.last().map(|(range, _)| range.end), Some(row.end));
            for pair in covered.windows(2) {
                prop_assert_eq!(pair[0].0.end, pair[1].0.start);
                prop_assert_ne!(pair[0].1, pair[1].1);
            }
        }
    }
}
