use super::*;
use crate::line::Span;

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
fn takes_its_faintest_trace_from_the_teleprompter() {
    assert_eq!(Ink::Trace.rgb(), [15, 15, 15]);
}

#[test]
fn shades_an_ink_by_its_heat_then_its_alpha() {
    assert_eq!(shade(Ink::Full, 0, OPAQUE), [255, 255, 255]);
    assert_eq!(shade(Ink::Full, 0, 0), [0, 0, 0]);
    assert_eq!(shade(Ink::Full, 255, 128), [128, 45, 16]);
    assert_eq!(
        shade(Ink::Full, 128, 255),
        Span::new("x", Ink::Full).heated(128).rgb()
    );
}

#[test]
fn turns_a_share_into_an_alpha_within_its_range() {
    assert_eq!(opacity(0.0), 0);
    assert_eq!(opacity(1.0), 255);
    assert_eq!(opacity(0.5), 128);
    assert_eq!(opacity(-2.0), 0);
    assert_eq!(opacity(3.0), 255);
    assert_eq!(opacity(f64::NAN), 0);
}
