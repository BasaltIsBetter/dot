use super::*;
use crate::ink::{Ink, OPAQUE};

#[test]
fn keeps_the_text_and_ink_of_a_span() {
    let span = Span::new("Dot", Ink::Live);

    assert_eq!(span.text, "Dot");
    assert_eq!(span.ink, Ink::Live);
}

#[test]
fn a_span_takes_a_column_per_character() {
    assert_eq!(Span::new("été", Ink::Full).columns(), 3);
    assert_eq!(Span::new("", Ink::Full).columns(), 0);
    assert_eq!(Span::new("⌘ ls", Ink::Full).columns(), 4);
}

#[test]
fn keeps_the_indent_and_spans_of_a_line() {
    let line = Line::new(12, vec![Span::new("Dot", Ink::Full)]);

    assert_eq!(line.indent, 12);
    assert_eq!(line.spans, vec![Span::new("Dot", Ink::Full)]);
    assert_eq!(line.alpha, OPAQUE);
    assert_eq!(Line::default(), Line::new(0, Vec::new()));
    assert_eq!(Line::default().alpha, 255);
}

#[test]
fn a_faded_line_keeps_the_share_of_its_ink_it_is_given() {
    let line = Line::new(0, vec![Span::new("Dot", Ink::Full)]);

    assert_eq!(line.clone().faded(1.0).alpha, 255);
    assert_eq!(line.clone().faded(0.5).alpha, 128);
    assert_eq!(line.clone().faded(0.0).alpha, 0);
    assert_eq!(line.clone().faded(0.5).faded(0.5).alpha, 64);
    assert_eq!(line.faded(0.64).spans, vec![Span::new("Dot", Ink::Full)]);
}

#[test]
fn a_faded_line_draws_its_spans_darker_on_black() {
    let line = Line::new(0, vec![Span::new("Dot", Ink::Full)]).faded(0.64);

    assert_eq!(
        line.rgb_of(&line.spans[0], &Palette::default()),
        Ink::Secondary.rgb()
    );
    assert_eq!(
        Line::new(0, Vec::new()).rgb_of(&Span::new("Dot", Ink::Live), &Palette::default()),
        Ink::Live.rgb()
    );
}

#[test]
fn a_cold_span_keeps_the_colour_of_its_ink() {
    assert_eq!(
        Span::new("Dot", Ink::Receding).rgb(&Palette::default()),
        Ink::Receding.rgb()
    );
    assert_eq!(Span::new("Dot", Ink::Full).heat, 0);
}

#[test]
fn a_hot_span_turns_the_live_colour() {
    assert_eq!(
        Span::new("Dot", Ink::Full)
            .heated(u8::MAX)
            .rgb(&Palette::default()),
        Ink::Live.rgb()
    );
    assert_eq!(
        Span::new("Dot", Ink::Receding)
            .heated(u8::MAX)
            .rgb(&Palette::default()),
        Ink::Live.rgb()
    );
}

#[test]
fn a_cooling_span_mixes_its_ink_with_the_live_colour() {
    assert_eq!(
        Span::new("Dot", Ink::Full)
            .heated(128)
            .rgb(&Palette::default()),
        [212, 183, 255]
    );
    assert_eq!(
        Span::new("Dot", Ink::Receding)
            .heated(51)
            .rgb(&Palette::default()),
        [115, 104, 133]
    );
}

#[test]
fn tells_whether_a_line_has_anything_to_draw() {
    assert!(!Line::default().has_text());
    assert!(!Line::new(4, vec![Span::new("", Ink::Full)]).has_text());
    assert!(Line::new(0, vec![Span::new("", Ink::Full), Span::new("x", Ink::Full)]).has_text());
    assert!(
        !Line::new(0, vec![Span::new("x", Ink::Full)])
            .faded(0.0)
            .has_text()
    );
    assert!(
        Line::new(0, vec![Span::new("x", Ink::Full)])
            .faded(0.01)
            .has_text()
    );
}

#[test]
fn keeps_a_short_text_whole() {
    assert_eq!(truncated("feat/mac-app", 24), "feat/mac-app");
    assert_eq!(truncated("abcd", 4), "abcd");
}

#[test]
fn ends_a_long_text_with_an_ellipsis_within_its_width() {
    assert_eq!(truncated("abcdef", 4), "abc…");
    assert_eq!(truncated("éééé", 3), "éé…");
    assert_eq!(truncated("ab", 1), "…");
    assert_eq!(truncated("ab", 0), "");
}

#[test]
fn a_text_width_is_the_widest_column_less_the_indent_by_default() {
    assert_eq!(TextWidth::default().columns(), 94);
    assert_eq!(TextWidth::new(28).columns(), 28);
}

proptest::proptest! {
    #[test]
    fn never_truncates_past_the_width(text in "\\PC{0,40}", longest in 0_usize..30) {
        proptest::prop_assert!(truncated(&text, longest).chars().count() <= longest);
    }
}

const BAND_ROW: f32 = 40.0;

#[test]
fn counts_the_rows_a_band_holds() {
    let band = Band {
        top: 100.0,
        bottom: 300.0,
    };

    assert!((band.rows(BAND_ROW) - 5.0).abs() < f64::EPSILON);
    assert!(band.rows(0.0).abs() < f64::EPSILON);
    assert!(
        Band {
            top: 300.0,
            bottom: 100.0
        }
        .rows(BAND_ROW)
        .abs()
            < f64::EPSILON
    );
    assert!((band.row_top(1.5, BAND_ROW) - 160.0).abs() < f32::EPSILON);
}

#[test]
fn cuts_a_text_onto_the_end_of_another() {
    let mut shown = "⌘ ".to_owned();

    push_truncated(&mut shown, "cargo test", 6);
    let cut = shown.clone();
    push_truncated(&mut shown, " ok", 3);
    let whole = shown.clone();
    push_truncated(&mut shown, "gone", 0);

    assert_eq!(cut, "⌘ cargo…");
    assert_eq!(whole, "⌘ cargo… ok");
    assert_eq!(shown, whole);
}
