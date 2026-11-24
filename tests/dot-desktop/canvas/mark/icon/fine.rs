use winit::dpi::PhysicalSize;

use super::*;
use dot_transcript::{Ink, OPAQUE};

fn layout_at(scale: f64) -> Layout {
    Layout::reflowed(PhysicalSize::new(2000, 1400), scale)
}

fn paint() -> Paint {
    Paint::shaded(Ink::Full, 0, OPAQUE)
}

fn bounds(drawn: &[Rectangle]) -> [f32; 4] {
    let left = drawn.iter().map(|mark| mark.left).fold(f32::MAX, f32::min);
    let top = drawn.iter().map(|mark| mark.top).fold(f32::MAX, f32::min);
    let right = drawn
        .iter()
        .map(|mark| mark.left + mark.width)
        .fold(f32::MIN, f32::max);
    let bottom = drawn
        .iter()
        .map(|mark| mark.top + mark.height)
        .fold(f32::MIN, f32::max);
    [left, top, right, bottom]
}

#[test]
fn draws_the_arrow_that_opens_a_file_as_a_hairline_square_centred_in_its_cell() {
    let layout = layout_at(2.0);
    let side = layout.scaled(SIDE);

    let drawn = Fine::Opens.drawn([100.0, 200.0, 20.0, 40.0], &layout, paint());

    let [left, top, right, bottom] = bounds(&drawn);
    assert!((right - left - side).abs() < 0.01 && (bottom - top - side).abs() < 0.01);
    assert!((left - (100.0 + ((20.0 - side) / 2.0).round())).abs() < 0.01);
    assert!((top - (200.0 + ((40.0 - side) / 2.0).round())).abs() < 0.01);
    assert!(
        drawn
            .iter()
            .all(|mark| mark.width.min(mark.height) <= Layout::HAIRLINE)
    );
    assert!(drawn.iter().all(|mark| mark.color == paint()));
}

#[test]
fn draws_the_shaft_from_the_bottom_left_and_the_head_at_the_top_right() {
    let layout = layout_at(2.0);
    let side = layout.scaled(SIDE);
    let head = (side * HEAD_SHARE).round();

    let drawn = Fine::Opens.drawn([0.0, 0.0, side, side], &layout, paint());

    let has = |left: f32, top: f32| {
        drawn.iter().any(|mark| {
            left >= mark.left
                && left < mark.left + mark.width
                && top >= mark.top
                && top < mark.top + mark.height
        })
    };
    assert!(has(0.0, side - 1.0));
    assert!(has(side - 1.0, 0.0));
    assert!(has(side - head, 0.0) && !has(side - head - 1.0, 0.0));
    assert!(has(side - 1.0, head - 1.0) && !has(side - 1.0, head));
    assert!(!has(0.0, 0.0) && !has(side - 1.0, side - 1.0));
}

#[test]
fn keeps_its_size_in_points_on_a_standard_screen() {
    let retina = layout_at(2.0);
    let standard = layout_at(1.0);

    let big = bounds(&Fine::Opens.drawn([0.0, 0.0, 40.0, 40.0], &retina, paint()));
    let small = bounds(&Fine::Opens.drawn([0.0, 0.0, 40.0, 40.0], &standard, paint()));

    assert!(big[2] - big[0] > small[2] - small[0]);
    assert!(small[2] - small[0] >= 3.0 * Layout::HAIRLINE);
}

#[test]
fn draws_the_enter_arrow_lower_than_wide_with_its_head_on_the_left() {
    let layout = layout_at(2.0);
    let side = layout.scaled(SIDE);
    let tall = (side * ENTER_HEIGHT_SHARE).round();

    let drawn = Fine::Enter.drawn([0.0, 0.0, side, tall], &layout, paint());

    let [left, top, right, bottom] = bounds(&drawn);
    assert!((right - left - side).abs() < 0.01);
    assert!((bottom - top - tall).abs() < 0.01);
    let middle = (tall / 2.0).floor();
    let near = |value: f32, wanted: f32| (value - wanted).abs() < 0.01;
    let has = |left: f32, top: f32| {
        drawn
            .iter()
            .any(|mark| near(mark.left, left) && near(mark.top, top))
    };
    assert!(
        drawn
            .iter()
            .any(|mark| near(mark.top, middle) && near(mark.width, side))
    );
    assert!(has(side - 1.0, 0.0));
    assert!(has(1.0, middle - 1.0));
    assert!(has(1.0, middle + 1.0));
    assert!(has(middle, 0.0));
    assert!(has(middle, 2.0 * middle));
    assert!(!has(middle + 1.0, 2.0 * middle + 1.0));
}

#[test]
fn keeps_three_pixels_at_least_on_the_smallest_screen() {
    let tiny = layout_at(0.5);

    let [left, _, right, _] = bounds(&Fine::Opens.drawn([0.0, 0.0, 40.0, 40.0], &tiny, paint()));

    assert!((right - left - 3.0).abs() < 0.01, "{}", right - left);
}
