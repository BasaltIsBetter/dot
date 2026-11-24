use std::time::Duration;

use super::*;
use crate::ink::OPAQUE;
use crate::line::{Band, TextWidth};
use crate::transcript::tool::{Aside, Glyph, Tool};

#[test]
fn draws_each_kind_of_entry_in_its_own_ink() {
    assert_eq!(ink_of(Kind::Prompt), Ink::Full);
    assert_eq!(ink_of(Kind::Note), Ink::Secondary);
    assert_eq!(ink_of(Kind::Answer), Ink::Full);
    assert_eq!(ink_of(Kind::Thinking), Ink::Secondary);
    assert_eq!(ink_of(Kind::Attachment), Ink::Secondary);
    assert_eq!(ink_of(Kind::Tool), Ink::Secondary);
    assert_eq!(ink_of(Kind::Notice), Ink::Receding);
    assert_eq!(marker_ink(Kind::Prompt), Ink::Receding);
    assert_eq!(marker_ink(Kind::Note), Ink::Secondary);
}

#[test]
fn starts_a_piece_past_the_indent_of_the_text() {
    assert_eq!(indent_at(0), TEXT_INDENT);
    assert_eq!(indent_at(5), TEXT_INDENT + 5);
    assert_eq!(indent_at(usize::MAX), u16::MAX);
}

#[test]
fn cuts_a_row_into_spans_heated_by_their_arrival() {
    let start = Instant::now();
    let mut entry = Entry::new(Kind::Thinking, "abc", start, TextWidth::default());
    entry.extend(
        "def",
        start + Duration::from_millis(600),
        TextWidth::default(),
    );

    let spans = pieces(&entry, 0..6, start + Duration::from_millis(600));

    assert_eq!(
        spans,
        vec![
            Span::new("abc", Ink::Secondary).heated(80),
            Span::new("def", Ink::Secondary).heated(u8::MAX),
        ]
    );
}

fn line_of(text: &str, start: Instant) -> Entry {
    let mut entry = Entry::tool(
        Tool {
            glyph: Glyph::Command,
            label: "cargo".to_owned(),
        },
        start,
    );
    entry.end(start);
    entry.sum_up(
        Aside {
            cut: "",
            kept: text,
        },
        start,
        TextWidth::default(),
    );
    entry
}

fn replaced(before: &str, now: &str, start: Instant, width: TextWidth) -> Entry {
    let mut entry = line_of(before, start);
    let aside = Aside { cut: "", kept: now };
    entry.replace_aside(aside, start + Duration::from_secs(2), width);
    entry
}

fn drawn(entry: &Entry, caret: bool, since_replaced: u64, start: Instant) -> Vec<Placed> {
    let place = Place {
        band: Band {
            top: 0.0,
            bottom: 1_000.0,
        },
        line_height: 20.0,
        first: 0,
        offset: 0.0,
    };
    let text_row = TextRow {
        entry,
        row: 0,
        top: 100.0,
        rolling: None,
        caret,
        marker: None,
        alpha: 1.0,
    };
    let now = start + Duration::from_secs(2) + Duration::from_millis(since_replaced);
    let mut placed = Vec::new();
    push_row(&mut placed, text_row, place, now);
    placed
}

fn text_of(placed: &Placed) -> String {
    placed
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

const TESTED: &str = "⌘ cargo test  ✓ · 0,8 s";
const BUILT: &str = "⌘ cargo build  ✓ · 0,8 s";

#[test]
fn only_the_words_that_changed_move_when_a_command_line_is_replaced() {
    let start = Instant::now();
    let entry = replaced(TESTED, BUILT, start, TextWidth::default());

    let placed = drawn(&entry, false, 100, start);

    assert_eq!(placed.len(), 3, "{placed:?}");
    let [still, rising, leaving] = [&placed[0], &placed[1], &placed[2]];
    assert_eq!(text_of(still), "⌘ cargo        ✓ · 0,8 s");
    assert_eq!((still.top, still.line.indent), (100.0, TEXT_INDENT));
    assert_eq!(text_of(rising), "build");
    assert_eq!(text_of(leaving), "test");
    assert_eq!(rising.line.indent, TEXT_INDENT + 8);
    assert_eq!(leaving.line.indent, TEXT_INDENT + 8);
    assert!(rising.top > still.top && leaving.top < still.top);
    assert!(rising.line.alpha < OPAQUE && leaving.line.alpha < OPAQUE);
}

#[test]
fn the_words_that_changed_arrive_orange_and_cool_while_the_rest_stays_put() {
    let start = Instant::now();
    let entry = replaced(TESTED, BUILT, start, TextWidth::default());

    let moving = drawn(&entry, false, 100, start);
    let settled = drawn(&entry, false, 600, start);

    assert!(moving[1].line.spans[0].heat > 200);
    assert_eq!(moving[2].line.spans[0].heat, 0);
    assert!(moving[0].line.spans.iter().all(|span| span.heat == 0));
    assert_eq!(settled.len(), 1);
    let heats: Vec<u8> = settled[0].line.spans.iter().map(|span| span.heat).collect();
    assert_eq!(text_of(&settled[0]), BUILT);
    assert_eq!(heats.len(), 3);
    assert!(heats[0] == 0 && heats[1] > 0 && heats[1] < u8::MAX && heats[2] == 0);
}

#[test]
fn leaves_out_the_part_that_did_not_change_when_there_is_none() {
    let start = Instant::now();
    let entry = replaced("⌘ a", "⌘ b", start, TextWidth::default());

    let settled = drawn(&entry, false, 600, start);

    let texts: Vec<&str> = settled[0]
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(texts, ["⌘ ", "b"]);
}

#[test]
fn once_cooled_a_replaced_line_is_drawn_like_any_other() {
    let start = Instant::now();
    let entry = replaced(TESTED, BUILT, start, TextWidth::default());

    let placed = drawn(&entry, false, 1_300, start);

    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].line.spans, [Span::new(BUILT, Ink::Secondary)]);
}

#[test]
fn ends_a_replaced_line_with_the_caret_when_it_is_lit() {
    let start = Instant::now();
    let entry = replaced(TESTED, BUILT, start, TextWidth::default());

    let moving = drawn(&entry, true, 100, start);
    let settled = drawn(&entry, true, 600, start);
    let unlit = drawn(&entry, false, 600, start);

    assert!(text_of(&moving[0]).ends_with(" s_"));
    assert!(text_of(&settled[0]).ends_with(" s_"));
    assert!(text_of(&unlit[0]).ends_with(" s"));
}

#[test]
fn a_replaced_line_on_several_rows_is_drawn_like_any_other() {
    let start = Instant::now();
    let entry = replaced(TESTED, BUILT, start, TextWidth::new(10));

    let placed = drawn(&entry, false, 100, start);

    assert_eq!(entry.row_count(), 3);
    assert_eq!(placed.len(), 1);
}
