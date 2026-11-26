use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::Paint;
use dot_transcript::Ink;

const RETINA: f64 = 2.0;

fn layout_of(width: u32) -> Layout {
    Layout::reflowed(PhysicalSize::new(width, 1800), RETINA)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.01
}

#[test]
fn centres_six_hundred_points_on_the_window() {
    let layout = layout_of(2880);

    let frame = Frame::of(&layout, 900.0, 1700.0);

    assert!(close(frame.width, (layout.scaled(600.0)).round()));
    assert!(close(frame.left + frame.width / 2.0, 1440.0));
    assert!(close(frame.right(), frame.left + frame.width));
    assert!(close(
        frame.band_top,
        (900.0 - layout.scaled(30.0) / 2.0).round()
    ));
    assert!(close(frame.floor, 1700.0));
}

#[test]
fn keeps_forty_points_on_each_side_in_a_narrow_window() {
    let layout = layout_of(1000);

    let frame = Frame::of(&layout, 900.0, 1700.0);

    assert!(close(frame.left, layout.scaled(40.0).round()));
    assert!(close(frame.right(), 1000.0 - layout.scaled(40.0).round()));
}

#[test]
fn shows_nothing_wide_in_a_window_narrower_than_its_sides() {
    let layout = layout_of(100);

    let frame = Frame::of(&layout, 900.0, 1700.0);

    assert!(close(frame.width, 0.0));
}

#[test]
fn opens_a_gap_before_each_section_but_the_first() {
    let layout = layout_of(2880);
    let row = layout.scaled(ROW);
    let gap = layout.scaled(SECTION_GAP);

    let offsets = offsets(&layout);

    assert_eq!(offsets.len(), ENTRIES.len());
    assert!(close(offsets[0], 0.0));
    assert!(close(offsets[1], row));
    assert!(close(offsets[5], 5.0 * row));
    assert!(close(offsets[6], 6.0 * row + gap));
    assert!(close(offsets[15], 15.0 * row + gap));
    assert!(close(offsets[16], 16.0 * row + 2.0 * gap));
    assert!(close(offsets[21], 21.0 * row + 2.0 * gap));
    assert!(close(offsets[22], 22.0 * row + 3.0 * gap));
    assert!(close(offsets[25], 25.0 * row + 3.0 * gap));
}

#[test]
fn reads_the_offset_between_two_places() {
    let offsets = [0.0, 10.0, 30.0];

    assert!(close(interpolated(&offsets, 0.0), 0.0));
    assert!(close(interpolated(&offsets, 1.5), 20.0));
    assert!(close(interpolated(&offsets, 2.0), 30.0));
    assert!(close(interpolated(&offsets, 7.0), 30.0));
    assert!(close(interpolated(&offsets, -1.0), 0.0));
    assert!(close(interpolated(&[], 1.0), 0.0));
}

#[test]
fn makes_room_for_the_hint_under_the_place_in_focus() {
    assert!(close(below(2, 2.0), 0.0));
    assert!(close(below(3, 2.0), 1.0));
    assert!(close(below(3, 2.5), 0.5));
    assert!(close(below(9, 2.0), 1.0));
    assert!(close(below(1, 2.0), 0.0));
}

#[test]
fn fades_the_rows_near_the_top_and_the_floor() {
    let floor = 1000.0;

    assert!((masked(500.0, floor) - 1.0).abs() < f64::EPSILON);
    assert!((masked(0.0, floor) - 0.0).abs() < f64::EPSILON);
    assert!((masked(80.0, floor) - 0.5).abs() < 1e-6);
    assert!((masked(920.0, floor) - 0.5).abs() < 1e-6);
    assert!((masked(1_200.0, floor) - 0.0).abs() < f64::EPSILON);
    assert!((masked(10.0, 0.0) - 0.0).abs() < f64::EPSILON);
}

#[test]
fn rings_a_square_with_four_hairlines_outside_it() {
    let paint = Paint::ink(Ink::Full);

    let ring = ring([10.0, 20.0, 10.0], 2.0, paint);

    assert_eq!(ring[0], rectangle([8.0, 18.0, 14.0, 1.0], paint));
    assert_eq!(ring[1], rectangle([8.0, 31.0, 14.0, 1.0], paint));
    assert_eq!(ring[2], rectangle([8.0, 18.0, 1.0, 14.0], paint));
    assert_eq!(ring[3], rectangle([21.0, 18.0, 1.0, 14.0], paint));
}

#[test]
fn takes_in_the_top_left_edges_but_not_the_bottom_right_ones() {
    let bounds = [10.0, 20.0, 5.0, 5.0];

    assert!(inside([10.0, 20.0], bounds));
    assert!(inside([14.9, 24.9], bounds));
    assert!(!inside([15.0, 22.0], bounds));
    assert!(!inside([12.0, 25.0], bounds));
    assert!(!inside([9.9, 22.0], bounds));
    assert!(!inside([12.0, 19.9], bounds));
}

#[test]
fn counts_one_column_per_character() {
    assert!(close(width_of("Modèle"), 6.0));
    assert!(close(width_of(""), 0.0));
}

#[test]
fn lays_a_line_in_a_pane_that_never_clips() {
    let pane = pane(10.4, TextSize::Small, 30.0, Line::new(0, Vec::new()));

    assert!(close(pane.left, 10.0));
    assert_eq!(pane.size, TextSize::Small);
    assert!(close(pane.lines[0].top, 30.0));
    assert_eq!(pane.lines[0].clip, None);
    assert!(pane.top < -1.0e30 && pane.bottom > 1.0e30);
}
