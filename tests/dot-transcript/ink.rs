use super::*;
use crate::line::Span;

#[test]
fn takes_its_inks_from_spaceship() {
    assert_eq!(Ink::Full.rgb(), [255, 255, 255]);
    assert_eq!(Ink::Secondary.rgb(), [163, 163, 163]);
    assert_eq!(Ink::Receding.rgb(), [102, 102, 102]);
    assert_eq!(Ink::Faint.rgb(), [56, 56, 56]);
    assert_eq!(Ink::Ground.rgb(), [26, 26, 26]);
    assert_eq!(Ink::Live.rgb(), [169, 112, 255]);
}

#[test]
fn takes_its_faintest_trace_from_the_teleprompter() {
    assert_eq!(Ink::Trace.rgb(), [15, 15, 15]);
}

const GREEN: [u8; 3] = [0x3d, 0xdc, 0x84];

#[test]
fn shades_an_ink_by_its_heat_then_its_alpha() {
    let palette = Palette::default();

    assert_eq!(palette.shade(Ink::Full, 0, OPAQUE), [255, 255, 255]);
    assert_eq!(palette.shade(Ink::Full, 0, 0), [0, 0, 0]);
    assert_eq!(palette.shade(Ink::Full, 255, 128), [85, 56, 128]);
    assert_eq!(
        palette.shade(Ink::Full, 128, 255),
        Span::new("x", Ink::Full).heated(128).rgb(&palette)
    );
}

#[test]
fn the_default_palette_is_dot_s_violet() {
    assert_eq!(Palette::default().live, Ink::Live.rgb());
    assert_eq!(Palette::default().rgb(Ink::Live), [0xa9, 0x70, 0xff]);
}

#[test]
fn draws_live_ink_in_the_live_colour_chosen() {
    let green = Palette { live: GREEN };

    assert_eq!(green.rgb(Ink::Live), GREEN);
    assert_eq!(green.shade(Ink::Live, 0, OPAQUE), GREEN);
}

#[test]
fn heat_mixes_toward_the_live_colour_chosen() {
    let green = Palette { live: GREEN };

    assert_eq!(green.shade(Ink::Full, 255, OPAQUE), GREEN);
    assert_eq!(green.shade(Ink::Full, 128, OPAQUE), [158, 237, 193]);
    assert_eq!(green.shade(Ink::Receding, 0, OPAQUE), Ink::Receding.rgb());
}

#[test]
fn an_ink_of_its_own_keeps_its_colour_in_any_palette() {
    let teal = Ink::Rgb([0x22, 0xd3, 0xee]);

    assert_eq!(teal.rgb(), [0x22, 0xd3, 0xee]);
    assert_eq!(Palette { live: GREEN }.rgb(teal), [0x22, 0xd3, 0xee]);
    assert_eq!(Palette::default().shade(teal, 0, 0), [0, 0, 0]);
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
