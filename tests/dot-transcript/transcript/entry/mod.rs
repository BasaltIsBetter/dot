use std::time::Duration;

use proptest::prelude::*;

use super::*;
use crate::line::WIDEST_COLUMN;
use crate::transcript::tool::{Aside, Glyph};

fn whole(line: &str) -> Aside<'_> {
    Aside {
        cut: "",
        kept: line,
    }
}

fn pieces(entry: &Entry, row: Range<usize>, now: Instant) -> Vec<(Range<usize>, u8)> {
    let mut pieces = Vec::new();
    entry.each_piece(row, now, |piece, heat| pieces.push((piece, heat)));
    pieces
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
fn says_whether_a_piece_added_anything() {
    let now = Instant::now();
    let mut entry = Entry::empty(Kind::Answer, width());

    assert!(!entry.extend("", now, width()));
    assert!(!entry.extend("\u{7}", now, width()));
    assert!(entry.extend("a", now, width()));
}

#[test]
fn an_empty_entry_has_one_empty_row_and_nothing_arrived() {
    let entry = Entry::empty(Kind::Answer, width());

    assert_eq!(entry.text, "");
    assert_eq!(entry.rows().collect::<Vec<_>>(), vec![0..0]);
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
fn rewraps_only_the_last_row_as_text_arrives() {
    let now = Instant::now();
    let mut entry = Entry::new(Kind::Answer, "abc\ndef", now, width());

    entry.extend(" more", now, width());

    assert_eq!(entry.rows().collect::<Vec<_>>(), vec![0..3, 4..12]);
}

#[test]
fn settles_once_its_last_piece_has_cooled() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Answer, &"word ".repeat(50), start, width());
    entry.extend("tail", start, width());

    let while_hot = entry.settle(after(start, 1_000));
    let rows = entry.rows().collect::<Vec<_>>();
    let once_cold = entry.settle(after(start, 1_300));

    assert!(!while_hot);
    assert!(once_cold);
    assert_eq!(entry.arrivals, []);
    assert_eq!(entry.arrivals.capacity(), 0);
    assert_eq!(entry.text.capacity(), entry.text.len());
    assert_eq!(entry.rows().collect::<Vec<_>>(), rows);
    assert_eq!(
        pieces(&entry, rows[0].clone(), after(start, 1_300)),
        vec![(rows[0].clone(), 0)]
    );
}

#[test]
fn a_command_line_settles_once_the_line_that_replaced_it_has_cooled() {
    let start = Instant::now();
    let mut entry = Entry::tool(tool(), start);
    entry.end(start);
    entry.sum_up(whole("⌘ a  ✓"), start, width());
    entry.replace_aside(whole("⌘ b  ✓"), after(start, 2_000), width());

    let while_hot = entry.settle(after(start, 2_500));
    let kept = entry.replaced().is_some();
    let once_cold = entry.settle(after(start, 3_300));

    assert!(!while_hot);
    assert!(kept);
    assert!(once_cold);
    assert_eq!(entry.replaced(), None);
    assert_eq!(entry.text, "⌘ b  ✓");
}

#[test]
fn an_entry_without_arrival_is_settled_at_once() {
    let mut entry = Entry::empty(Kind::Answer, width());

    assert!(entry.settle(Instant::now()));
}

#[test]
fn finds_the_row_holding_an_offset() {
    let entry = Entry::new(Kind::Answer, "abc\ndef\nghi", Instant::now(), width());

    assert_eq!(entry.row_holding(0), 0);
    assert_eq!(entry.row_holding(5), 1);
    assert_eq!(entry.row_holding(8), 2);
    assert_eq!(entry.row_count(), 3);
}

#[test]
fn cuts_a_piece_on_a_character_boundary() {
    assert_eq!(floor_char_boundary("aé", 2), 1);
    assert_eq!(floor_char_boundary("aé", 3), 3);
    assert_eq!(floor_char_boundary("aé", 9), 3);
    assert_eq!(floor_char_boundary("aé", 0), 0);
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
        prop_assert_eq!(entry.rows().collect::<Vec<_>>(), wrap(&whole, width().columns()));
    }
}

fn tool() -> Tool {
    Tool {
        glyph: Glyph::Command,
        label: "cargo test".to_owned(),
    }
}

#[test]
fn a_tool_takes_no_row_until_a_line_sums_it_up() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(), now);

    assert!(!entry.takes_rows());
    assert_eq!(entry.kind, Kind::Tool);
    assert_eq!(entry.mark.as_ref().map(|mark| mark.started), Some(now));
    entry.rewrap(width());
    assert!(!entry.takes_rows());

    entry.extend("cargo test  ok", now, width());

    assert!(entry.takes_rows());
    assert_eq!(entry.row_count(), 1);
}

#[test]
fn a_tool_wraps_the_line_that_sums_it_up_again_when_the_width_changes() {
    let now = Instant::now();
    let mut entry = Entry::tool(tool(), now);
    entry.extend("cargo test --workspace  ok · 12.4 s", now, width());

    entry.rewrap(TextWidth::new(10));

    assert_eq!(entry.row_count(), 4);
}

#[test]
fn shows_no_more_of_a_row_than_the_widest_column_can_hold() {
    let width = TextWidth::new(20);
    let text = format!("word{}", " ".repeat(500_000));
    let entry = Entry::new(Kind::Answer, &text, Instant::now(), width);

    let row = entry.row(0);
    let shown = entry.shown_row(0);

    assert_eq!(row.len(), text.len());
    assert_eq!(shown, 0..usize::from(WIDEST_COLUMN));
    assert_eq!(entry.shown_row(0), entry.shown_row(0));
}

#[test]
fn shows_a_short_row_whole() {
    let entry = Entry::new(
        Kind::Answer,
        "é😀 short",
        Instant::now(),
        TextWidth::new(40),
    );

    assert_eq!(entry.shown_row(0), entry.row(0));
}
