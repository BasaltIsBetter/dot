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

#[test]
fn gives_each_size_its_percentage_from_the_former_one_up() {
    let percents = InterfaceSize::ALL.map(InterfaceSize::percent);

    assert_eq!(percents, [80, 90, 100, 115, 130]);
}
