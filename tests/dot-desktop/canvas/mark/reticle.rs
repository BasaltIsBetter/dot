use super::*;
use crate::canvas::mark::shape::Rgba;

const RETINA: f64 = 2.0;
const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 300.0;

fn reticle_at(x: f64, y: f64) -> Reticle {
    Reticle::at(PhysicalPosition::new(x, y), RETINA)
}

fn hairline(left: f32, top: f32, width: f32, height: f32) -> Rectangle {
    Rectangle {
        left,
        top,
        width,
        height,
        color: Rgba::ink(Ink::Ground),
    }
}

#[test]
fn snaps_the_pointer_to_the_pixel_under_it() {
    let reticle = reticle_at(100.7, 50.2);

    assert_eq!(reticle.column, 100.0);
    assert_eq!(reticle.row, 50.0);
}

fn edge(left: f32, top: f32, width: f32, height: f32) -> Rectangle {
    Rectangle {
        left,
        top,
        width,
        height,
        color: Rgba::ink(Ink::Full),
    }
}

#[test]
fn draws_a_square_of_sixteen_points_across_centred_on_the_pixel() {
    let frame = reticle_at(100.7, 50.2).frame();

    assert_eq!(
        frame,
        [
            edge(84.0, 34.0, 33.0, 1.0),
            edge(84.0, 66.0, 33.0, 1.0),
            edge(84.0, 35.0, 1.0, 31.0),
            edge(116.0, 35.0, 1.0, 31.0),
        ]
    );
    assert_eq!(
        Reticle::at(PhysicalPosition::new(0.0, 0.0), 1.0).half_side,
        8.0
    );
}

#[test]
fn crosses_the_window_edge_to_edge_and_stops_at_the_square() {
    let hairlines: Vec<_> = reticle_at(100.0, 50.0).hairlines(WIDTH, HEIGHT).collect();

    assert_eq!(
        hairlines,
        [
            hairline(100.0, 0.0, 1.0, 34.0),
            hairline(100.0, 67.0, 1.0, 233.0),
            hairline(0.0, 50.0, 84.0, 1.0),
            hairline(117.0, 50.0, 283.0, 1.0),
        ]
    );
}

#[test]
fn leaves_the_inside_of_the_square_empty_and_meets_its_edges() {
    let reticle = reticle_at(100.0, 50.0);
    let [top, bottom, left, right] = reticle.frame();

    for hairline in reticle.hairlines(WIDTH, HEIGHT) {
        let inside_across =
            hairline.left + hairline.width > left.left + 1.0 && hairline.left < right.left;
        let inside_down =
            hairline.top + hairline.height > top.top + 1.0 && hairline.top < bottom.top;
        assert!(!(inside_across && inside_down), "{hairline:?}");
    }
    let hairlines: Vec<_> = reticle.hairlines(WIDTH, HEIGHT).collect();
    assert!((hairlines[0].top + hairlines[0].height - top.top).abs() < f32::EPSILON);
    assert!((hairlines[1].top - (bottom.top + 1.0)).abs() < f32::EPSILON);
}

#[test]
fn drops_the_hairlines_cut_off_by_a_window_edge() {
    let near_the_corner: Vec<_> = reticle_at(5.0, 5.0).hairlines(WIDTH, HEIGHT).collect();
    let on_the_far_edge: Vec<_> = reticle_at(399.0, 299.0).hairlines(WIDTH, HEIGHT).collect();

    assert_eq!(
        near_the_corner,
        [
            hairline(5.0, 22.0, 1.0, 278.0),
            hairline(22.0, 5.0, 378.0, 1.0)
        ]
    );
    assert_eq!(
        on_the_far_edge,
        [
            hairline(399.0, 0.0, 1.0, 283.0),
            hairline(0.0, 299.0, 383.0, 1.0)
        ]
    );
}

#[test]
fn drops_a_hairline_with_no_length_left_before_the_square() {
    let one_radius_from_the_corner: Vec<_> =
        reticle_at(16.0, 16.0).hairlines(WIDTH, HEIGHT).collect();

    assert_eq!(
        one_radius_from_the_corner,
        [
            hairline(16.0, 33.0, 1.0, 267.0),
            hairline(33.0, 16.0, 367.0, 1.0)
        ]
    );
}

const STRIP_TOP: f32 = 268.0;

#[test]
fn stops_the_vertical_hairline_at_the_top_of_the_status_bar() {
    let hairlines: Vec<_> = reticle_at(100.0, 50.0)
        .hairlines(WIDTH, STRIP_TOP)
        .collect();

    assert_eq!(hairlines[1], hairline(100.0, 67.0, 1.0, 201.0));
    assert_eq!(hairlines.len(), 4);
}

#[test]
fn draws_no_hairline_across_the_status_bar_when_the_pointer_is_over_it() {
    let hairlines: Vec<_> = reticle_at(100.0, 280.0)
        .hairlines(WIDTH, STRIP_TOP)
        .collect();

    assert_eq!(hairlines, [hairline(100.0, 0.0, 1.0, 264.0)]);
}

#[test]
fn keeps_the_horizontal_hairline_on_the_last_row_above_the_status_bar() {
    let hairlines: Vec<_> = reticle_at(100.0, 267.0)
        .hairlines(WIDTH, STRIP_TOP)
        .collect();

    assert!(hairlines.contains(&hairline(0.0, 267.0, 84.0, 1.0)));
    assert!(
        hairlines
            .iter()
            .all(|hairline| hairline.top + hairline.height <= STRIP_TOP)
    );
}

#[test]
fn draws_no_horizontal_hairline_on_the_top_row_of_the_status_bar() {
    let hairlines: Vec<_> = reticle_at(100.0, 268.0)
        .hairlines(WIDTH, STRIP_TOP)
        .collect();

    assert!(hairlines.iter().all(|hairline| hairline.height > 1.0));
}
