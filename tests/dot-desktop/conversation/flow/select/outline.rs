use std::collections::BTreeSet;

use dot_transcript::Ink;
use proptest::prelude::*;

use super::*;

const ROW: f32 = 6.0;
const ARM: f32 = 3.0;
const PAINT: Paint = Paint::ink(Ink::Full);
const WASH: f64 = 0.5;
const MARKS: Marks = Marks {
    color: PAINT,
    wash: WASH,
    arm: ARM,
};

fn spans_of(reaches: &[[u16; 2]]) -> Vec<RowSpan> {
    reaches
        .iter()
        .enumerate()
        .map(|(index, &[left, right])| {
            let top = f32::from(u16::try_from(index).unwrap()) * ROW;
            RowSpan {
                left: f32::from(left),
                right: f32::from(right),
                top,
                bottom: top + ROW,
            }
        })
        .collect()
}

fn outline_of(reaches: &[[u16; 2]]) -> Outline {
    Outline {
        spans: spans_of(reaches),
        opens: true,
        closes: true,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the tests draw on a small grid of whole pixels"
)]
fn pixels(rectangles: &[Rectangle]) -> BTreeSet<[i32; 2]> {
    rectangles
        .iter()
        .flat_map(|rectangle| {
            let [left, top] = [rectangle.left as i32, rectangle.top as i32];
            let [width, height] = [rectangle.width as i32, rectangle.height as i32];
            (left..left + width).flat_map(move |x| (top..top + height).map(move |y| [x, y]))
        })
        .collect()
}

fn ring_around(washed: &BTreeSet<[i32; 2]>) -> BTreeSet<[i32; 2]> {
    washed
        .iter()
        .flat_map(|&[x, y]| {
            (-1..=1).flat_map(move |across| (-1..=1).map(move |down| [x + across, y + down]))
        })
        .filter(|pixel| !washed.contains(pixel))
        .collect()
}

fn edge_inside(washed: &BTreeSet<[i32; 2]>) -> BTreeSet<[i32; 2]> {
    washed
        .iter()
        .filter(|&&[x, y]| {
            (-1..=1).any(|across| (-1..=1).any(|down| !washed.contains(&[x + across, y + down])))
        })
        .copied()
        .collect()
}

fn rings_of(outline: &Outline) -> [BTreeSet<[i32; 2]>; 2] {
    let mut marks = Vec::new();
    outline.frame(MARKS, &mut marks);
    let of = |share: f64| {
        let ring: Vec<Rectangle> = marks
            .iter()
            .filter(|mark| mark.color == PAINT.dimmed(opacity(share)))
            .copied()
            .collect();
        pixels(&ring)
    };
    [of(CONTOUR), of(CONTOUR + (1.0 - CONTOUR) * WASH)]
}

fn washed(outline: &Outline) -> BTreeSet<[i32; 2]> {
    pixels(&outline.wash(PAINT).collect::<Vec<_>>())
}

#[test]
fn fills_a_row_from_its_left_to_its_right() {
    let span = spans_of(&[[3, 10]])[0];

    let filled = span.filled(PAINT);

    assert_eq!(
        [filled.left, filled.top, filled.width, filled.height],
        [3.0, 0.0, 7.0, ROW]
    );
    let outline = outline_of(&[[3, 10], [0, 4]]);
    assert_eq!(outline.wash(PAINT).count(), 2);
}

#[test]
fn frames_one_row_with_four_corners_and_no_contour() {
    let outline = outline_of(&[[3, 13]]);
    let mut marks = Vec::new();

    outline.frame(MARKS, &mut marks);

    assert_eq!(marks.len(), 16);
    assert!(marks.iter().all(|mark| mark.color == PAINT));
    let corners: BTreeSet<[i32; 2]> = pixels(&marks);
    assert!(corners.contains(&[2, -1]) && corners.contains(&[13, -1]));
    assert!(corners.contains(&[2, 6]) && corners.contains(&[13, 6]));
    assert!(corners.contains(&[3, 0]) && corners.contains(&[12, 5]));
    assert!(!corners.contains(&[7, -1]) && !corners.contains(&[7, 0]));
}

#[test]
fn hugs_ragged_rows_with_a_contour_at_thirty_five_percent() {
    let outline = outline_of(&[[4, 20], [0, 14], [0, 6]]);

    let [outside, inside] = rings_of(&outline);

    assert!(outside.contains(&[20, 3]) && outside.contains(&[14, 9]) && outside.contains(&[6, 15]));
    assert!(!outside.contains(&[20, 9]) && !outside.contains(&[20, 15]));
    assert!(inside.contains(&[19, 3]) && inside.contains(&[19, 5]) && inside.contains(&[13, 9]));
    assert!(!inside.contains(&[10, 3]) && !inside.contains(&[10, 6]));
    assert_eq!(outside, ring_around(&washed(&outline)));
    assert_eq!(inside, edge_inside(&washed(&outline)));
}

#[test]
fn sets_the_top_corners_on_the_first_row_and_the_bottom_ones_on_the_last() {
    let outline = outline_of(&[[4, 20], [0, 14], [0, 6]]);
    let mut marks = Vec::new();

    outline.frame(MARKS, &mut marks);

    let corners = pixels(
        &marks
            .into_iter()
            .filter(|mark| mark.color == PAINT)
            .collect::<Vec<_>>(),
    );
    for corner in [[3, -1], [20, -1], [-1, 18], [6, 18]] {
        assert!(corners.contains(&corner), "{corner:?}");
    }
    assert!(!corners.contains(&[14, 12]) && !corners.contains(&[-1, -1]));
}

#[test]
fn leaves_out_the_corners_of_an_end_the_window_cuts() {
    let mut outline = outline_of(&[[4, 20], [0, 6]]);
    outline.opens = false;
    let mut marks = Vec::new();

    outline.frame(MARKS, &mut marks);

    let solid = marks.iter().filter(|mark| mark.color == PAINT).count();
    assert_eq!(solid, 8);
    assert!(
        marks
            .iter()
            .filter(|mark| mark.color == PAINT)
            .all(|mark| mark.top >= 6.0)
    );
    outline.closes = false;
    let mut none = Vec::new();
    outline.frame(MARKS, &mut none);
    assert!(none.iter().all(|mark| mark.color != PAINT));
}

#[test]
fn draws_nothing_for_no_row() {
    let mut marks = Vec::new();

    Outline::default().frame(MARKS, &mut marks);

    assert_eq!(marks.len(), 0);
}

#[test]
fn splits_the_contour_of_rows_that_do_not_touch() {
    let outline = outline_of(&[[20, 30], [0, 10]]);

    let [outside, inside] = rings_of(&outline);

    assert_eq!(outside, ring_around(&washed(&outline)));
    assert_eq!(inside, edge_inside(&washed(&outline)));
    assert!(!outside.contains(&[15, 5]) && !outside.contains(&[15, 6]));
}

proptest! {
    #[test]
    fn the_contour_is_the_two_rings_of_pixels_on_each_side_of_the_washed_edge(
        reaches in prop::collection::vec((0_u16..40, 1_u16..40), 2..7)
    ) {
        let reaches: Vec<[u16; 2]> = reaches
            .into_iter()
            .map(|(left, width)| [left, left + width])
            .collect();
        let outline = outline_of(&reaches);

        let [outside, inside] = rings_of(&outline);

        prop_assert_eq!(outside, ring_around(&washed(&outline)));
        prop_assert_eq!(inside, edge_inside(&washed(&outline)));
    }
}
