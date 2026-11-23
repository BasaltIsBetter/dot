use std::time::Duration;

use super::*;
use crate::line::TextWidth;

#[test]
fn draws_each_kind_of_entry_in_its_own_ink() {
    assert_eq!(ink_of(Kind::Prompt), Ink::Full);
    assert_eq!(ink_of(Kind::Note), Ink::Full);
    assert_eq!(ink_of(Kind::Answer), Ink::Full);
    assert_eq!(ink_of(Kind::Thinking), Ink::Secondary);
    assert_eq!(ink_of(Kind::Attachment), Ink::Secondary);
    assert_eq!(ink_of(Kind::Tool), Ink::Secondary);
    assert_eq!(ink_of(Kind::Notice), Ink::Receding);
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
