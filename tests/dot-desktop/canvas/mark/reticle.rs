use super::*;

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
        ink: Ink::Ground,
    }
}

#[test]
fn snaps_the_pointer_to_the_pixel_under_it() {
    let reticle = reticle_at(100.7, 50.2);

    assert_eq!(reticle.column, 100.0);
    assert_eq!(reticle.row, 50.0);
}

#[test]
fn draws_a_ring_of_sixteen_points_across_centred_on_the_pixel() {
    let ring = reticle_at(100.7, 50.2).ring();

    assert_eq!(
        ring,
        Ring {
            centre_x: 100.5,
            centre_y: 50.5,
            radius: 16.0,
            thickness: 1.0,
            ink: Ink::Full,
        }
    );
    assert_eq!(
        Reticle::at(PhysicalPosition::new(0.0, 0.0), 1.0).radius,
        8.0
    );
}

#[test]
fn crosses_the_window_edge_to_edge_and_stops_at_the_ring() {
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
fn leaves_the_inside_of_the_ring_empty() {
    let reticle = reticle_at(100.0, 50.0);
    let ring = reticle.ring();
    let inner = ring.radius - ring.thickness / 2.0;

    for hairline in reticle.hairlines(WIDTH, HEIGHT) {
        let nearest_x = ring
            .centre_x
            .clamp(hairline.left, hairline.left + hairline.width);
        let nearest_y = ring
            .centre_y
            .clamp(hairline.top, hairline.top + hairline.height);
        let distance = (nearest_x - ring.centre_x).hypot(nearest_y - ring.centre_y);
        assert!(distance >= inner, "{hairline:?}");
    }
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
fn drops_a_hairline_with_no_length_left_before_the_ring() {
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
