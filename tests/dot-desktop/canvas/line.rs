use super::*;

#[test]
fn takes_its_inks_from_spaceship() {
    assert_eq!(Ink::Full.rgb(), [255, 255, 255]);
    assert_eq!(Ink::Secondary.rgb(), [163, 163, 163]);
    assert_eq!(Ink::Receding.rgb(), [102, 102, 102]);
    assert_eq!(Ink::Faint.rgb(), [56, 56, 56]);
    assert_eq!(Ink::Ground.rgb(), [26, 26, 26]);
    assert_eq!(Ink::Live.rgb(), [255, 90, 31]);
}

#[test]
fn keeps_the_text_and_ink_of_a_span() {
    let span = Span::new("Dot", Ink::Live);

    assert_eq!(span.text, "Dot");
    assert_eq!(span.ink, Ink::Live);
}

#[test]
fn keeps_the_indent_and_spans_of_a_line() {
    let line = Line::new(12, vec![Span::new("Dot", Ink::Full)]);

    assert_eq!(line.indent, 12);
    assert_eq!(line.spans, vec![Span::new("Dot", Ink::Full)]);
    assert_eq!(Line::default(), Line::new(0, Vec::new()));
}

#[test]
fn a_cold_span_keeps_the_colour_of_its_ink() {
    assert_eq!(Span::new("Dot", Ink::Receding).rgb(), Ink::Receding.rgb());
    assert_eq!(Span::new("Dot", Ink::Full).heat, 0);
}

#[test]
fn a_hot_span_turns_orange() {
    assert_eq!(
        Span::new("Dot", Ink::Full).heated(u8::MAX).rgb(),
        Ink::Live.rgb()
    );
    assert_eq!(
        Span::new("Dot", Ink::Receding).heated(u8::MAX).rgb(),
        Ink::Live.rgb()
    );
}

#[test]
fn a_cooling_span_mixes_its_ink_with_orange() {
    assert_eq!(
        Span::new("Dot", Ink::Full).heated(128).rgb(),
        [255, 172, 143]
    );
    assert_eq!(
        Span::new("Dot", Ink::Receding).heated(51).rgb(),
        [133, 100, 88]
    );
}

#[test]
fn tells_whether_a_line_has_anything_to_draw() {
    assert!(!Line::default().has_text());
    assert!(!Line::new(4, vec![Span::new("", Ink::Full)]).has_text());
    assert!(Line::new(0, vec![Span::new("", Ink::Full), Span::new("x", Ink::Full)]).has_text());
}
