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
