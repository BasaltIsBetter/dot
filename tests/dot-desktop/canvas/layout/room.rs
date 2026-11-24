use super::*;

#[test]
fn has_the_sessions_column_from_eleven_hundred_points() {
    assert_eq!(Room::of(1_100.0, true), Room::Railed);
    assert_eq!(Room::of(1_099.9, true), Room::Wide);
    assert!(Room::Railed.has_rail());
    assert!(!Room::Wide.has_rail());
}

#[test]
fn has_the_gutter_from_nine_hundred_points() {
    assert_eq!(Room::of(900.0, true), Room::Wide);
    assert_eq!(Room::of(899.9, true), Room::Narrow);
    assert!(Room::Wide.has_gutter());
    assert!(Room::Railed.has_gutter());
    assert!(!Room::Narrow.has_gutter());
}

#[test]
fn keeps_the_teleprompter_margins_on_each_side() {
    assert_eq!(Room::Narrow.reserves(), [48.0, 48.0]);
    assert_eq!(Room::Wide.reserves(), [190.0, 190.0]);
    assert_eq!(Room::Railed.reserves(), [200.0, 440.0]);
}

#[test]
fn shifts_the_column_left_to_share_the_free_room_like_the_reserves() {
    assert!((shift(640.0, [200.0, 440.0]) - (200.0 - 320.0)).abs() < f32::EPSILON);
    assert!(shift(400.0, [190.0, 190.0]).abs() < f32::EPSILON);
    assert!(shift(400.0, [440.0, 200.0]).abs() < f32::EPSILON);
}

#[test]
fn never_shows_the_sessions_column_when_the_user_says_never() {
    assert_eq!(Room::of(1_100.0, false), Room::Wide);
    assert_eq!(Room::of(4_000.0, false), Room::Wide);
    assert_eq!(Room::of(899.9, false), Room::Narrow);
}
