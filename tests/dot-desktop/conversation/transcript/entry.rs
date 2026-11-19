use std::time::Duration;

use proptest::prelude::*;

use super::*;

impl Entry {
    fn pieces(&self, row: Range<usize>, now: Instant) -> Vec<(Range<usize>, u8)> {
        let mut pieces = Vec::new();
        self.each_piece(row, now, |piece, heat| pieces.push((piece, heat)));
        pieces
    }
}

fn width() -> TextWidth {
    TextWidth::default()
}

fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

#[test]
fn a_new_entry_is_wrapped_and_arrives_whole() {
    let now = Instant::now();

    let entry = Entry::new(Kind::Answer, "hello\nworld", now, width());

    assert_eq!(entry.rows, vec![0..5, 6..11]);
    assert_eq!(entry.arrivals, vec![Arrival { start: 0, at: now }]);
    assert_eq!(entry.last_arrival(), Some(now));
}

#[test]
fn an_empty_entry_has_one_empty_row_and_nothing_arrived() {
    let entry = Entry::empty(Kind::Answer, width());

    assert!(entry.is_empty());
    assert_eq!(entry.rows, vec![0..0]);
    assert_eq!(entry.last_arrival(), None);
}

#[test]
fn grows_by_the_pieces_that_arrive() {
    let start = Instant::now();
    let mut entry = Entry::empty(Kind::Answer, width());

    entry.extend("Bon", start, width());
    entry.extend("jour", after(start, 50), width());

    assert_eq!(entry.text, "Bonjour");
    assert_eq!(entry.rows, vec![0..7]);
    assert_eq!(
        entry.arrivals,
        vec![
            Arrival {
                start: 0,
                at: start
            },
            Arrival {
                start: 3,
                at: after(start, 50)
            },
        ]
    );
}

#[test]
fn cleans_what_the_model_writes() {
    let mut entry = Entry::empty(Kind::Answer, width());

    entry.extend("a\tb\u{1b}[31mc\r\nd", Instant::now(), width());

    assert_eq!(entry.text, "a    b[31mc\nd");
}

#[test]
fn ignores_a_piece_with_nothing_to_show() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "ok", start, width());

    entry.extend("\u{7}", after(start, 10), width());

    assert_eq!(entry.text, "ok");
    assert_eq!(entry.arrivals.len(), 1);
}

#[test]
fn forgets_when_settled_pieces_arrived() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "old", start, width());

    entry.extend(" new", after(start, 2_000), width());

    assert_eq!(
        entry.arrivals,
        vec![Arrival {
            start: 3,
            at: after(start, 2_000)
        }]
    );
}

#[test]
fn splits_a_row_where_its_pieces_arrived() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "abc", start, width());
    entry.extend("def", after(start, 600), width());

    let pieces = entry.pieces(0..6, after(start, 600));

    assert_eq!(pieces, vec![(0..3, 80), (3..6, u8::MAX)]);
}

#[test]
fn keeps_a_settled_row_in_one_piece() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "abc", start, width());
    entry.extend("def", after(start, 100), width());

    let pieces = entry.pieces(0..6, after(start, 5_000));

    assert_eq!(pieces, vec![(0..6, 0)]);
}

#[test]
fn heats_a_row_after_the_last_piece_that_started_before_it() {
    let start = Instant::now();
    let long = "word ".repeat(20);
    let entry = Entry::new(Kind::Answer, &long, start, width());
    let second_row = entry.rows[1].clone();

    let pieces = entry.pieces(second_row.clone(), start);

    assert_eq!(pieces, vec![(second_row, u8::MAX)]);
}

proptest! {
    #[test]
    fn wraps_pieces_as_it_would_wrap_the_whole_text(
        pieces in proptest::collection::vec("[a-z \n]{0,40}", 1..30),
    ) {
        let now = Instant::now();
        let mut entry = Entry::empty(Kind::Answer, width());
        for piece in &pieces {
            entry.extend(piece, now, width());
        }

        let whole: String = pieces.concat();
        prop_assert_eq!(&entry.text, &whole);
        prop_assert_eq!(entry.rows, wrap(&whole, width().columns()));
    }

    #[test]
    fn covers_each_row_with_its_pieces(
        pieces in proptest::collection::vec("[a-z ]{1,30}", 1..10),
        elapsed in 0_u64..2_000,
    ) {
        let start = Instant::now();
        let mut entry = Entry::empty(Kind::Answer, width());
        for (index, piece) in pieces.iter().enumerate() {
            entry.extend(piece, after(start, 100 * u64::try_from(index).unwrap()), width());
        }

        for row in entry.rows.clone() {
            let covered = entry.pieces(row.clone(), after(start, elapsed));
            prop_assert_eq!(covered.first().map(|(range, _)| range.start), Some(row.start));
            prop_assert_eq!(covered.last().map(|(range, _)| range.end), Some(row.end));
            for pair in covered.windows(2) {
                prop_assert_eq!(pair[0].0.end, pair[1].0.start);
                prop_assert_ne!(pair[0].1, pair[1].1);
            }
        }
    }
}
