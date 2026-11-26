use super::*;

#[test]
fn draws_four_corners_of_two_strokes_each_inside_the_card() {
    let colour = Paint::ink(Ink::Full);

    let strokes = corners([10.0, 20.0, 100.0, 50.0], 9.0, colour);

    let lefts: Vec<f32> = strokes.iter().map(|stroke| stroke.left).collect();
    let tops: Vec<f32> = strokes.iter().map(|stroke| stroke.top).collect();
    assert_eq!(lefts, [10.0, 10.0, 101.0, 109.0, 10.0, 10.0, 101.0, 109.0]);
    assert_eq!(tops, [20.0, 20.0, 20.0, 20.0, 69.0, 61.0, 69.0, 61.0]);
    let sizes: Vec<[f32; 2]> = strokes
        .iter()
        .map(|stroke| [stroke.width, stroke.height])
        .collect();
    assert_eq!(sizes, [[9.0, 1.0], [1.0, 9.0]].repeat(4));
}

#[test]
fn keeps_each_arm_of_a_corner_inside_a_box_smaller_than_the_arm() {
    let strokes = corners([10.0, 20.0, 6.0, 40.0], 9.0, Paint::ink(Ink::Full));

    let lefts: Vec<f32> = strokes.iter().map(|stroke| stroke.left).collect();
    let widths: Vec<f32> = strokes.iter().map(|stroke| stroke.width).collect();
    assert_eq!(lefts, [10.0, 10.0, 10.0, 15.0, 10.0, 10.0, 10.0, 15.0]);
    assert_eq!(widths, [6.0, 1.0, 6.0, 1.0, 6.0, 1.0, 6.0, 1.0]);
    let short = corners([0.0, 0.0, 40.0, 5.0], 9.0, Paint::ink(Ink::Full));
    assert_eq!(short[1].height, 5.0);
    assert_eq!(short[5].top, 0.0);
}

#[test]
fn clips_a_rectangle_to_a_band_and_drops_it_outside() {
    let mut rectangle = Rectangle {
        left: 0.0,
        top: 10.0,
        width: 4.0,
        height: 20.0,
        color: Paint::ink(Ink::Full),
    };

    assert!(rectangle.clip_to([15.0, 25.0]));
    assert_eq!((rectangle.top, rectangle.height), (15.0, 10.0));
    assert!(rectangle.clip_to([0.0, 100.0]));
    assert_eq!((rectangle.top, rectangle.height), (15.0, 10.0));
    assert!(!rectangle.clip_to([25.0, 30.0]));
    let mut above = rectangle;
    assert!(!above.clip_to([0.0, 15.0]));
}
