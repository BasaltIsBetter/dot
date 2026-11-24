use super::*;

#[test]
fn paints_each_hue_like_the_prototype() {
    let colours: Vec<_> = Hue::ALL.iter().map(|hue| hue.rgb()).collect();

    assert_eq!(
        colours,
        [
            [0xff, 0x5a, 0x1f],
            [0xff, 0x3b, 0x3b],
            [0xff, 0xb0, 0x20],
            [0x3d, 0xdc, 0x84],
            [0x22, 0xd3, 0xee],
            [0x4d, 0x7c, 0xff],
            [0xa9, 0x70, 0xff],
            [0xff, 0x4f, 0xa3],
        ]
    );
}

#[test]
fn gives_the_first_hue_nobody_has() {
    assert_eq!(Hue::first_free(&[], 0), Hue::Orange);
    assert_eq!(Hue::first_free(&[Hue::Orange, Hue::Amber], 2), Hue::Red);
}

#[test]
fn goes_round_the_hues_once_all_are_taken() {
    let taken = Hue::ALL;

    assert_eq!(Hue::first_free(&taken, 8), Hue::Orange);
    assert_eq!(Hue::first_free(&taken, 9), Hue::Red);
    assert_eq!(Hue::first_free(&taken, 15), Hue::Pink);
}

#[test]
fn a_toggle_shows_its_colour_only_when_on() {
    assert!(Toggle::On.is_on());
    assert!(!Toggle::Off.is_on());
}

#[test]
fn counts_the_characters_of_each_column() {
    assert_eq!(Column::Narrow.characters(), 64);
    assert_eq!(Column::Medium.characters(), 80);
    assert_eq!(Column::Wide.characters(), 96);
}

#[test]
fn places_the_reading_line_halfway_or_a_third_down() {
    assert!((ReadingLine::Middle.share() - 0.5).abs() < f32::EPSILON);
    assert!((ReadingLine::Third.share() - 1.0 / 3.0).abs() < f32::EPSILON);
}
