use super::*;
use dot_transcript::{Ink, OPAQUE};

const SCREEN: f32 = 1.0;
const RETINA: f32 = 2.0;

fn covers(silhouette: &Silhouette, row: u16) -> Vec<u8> {
    let columns = usize::from(pixels(outline::WIDTH * f64::from(silhouette.scale)));
    let mut covers = vec![0; columns];
    for run in silhouette.runs.iter().filter(|run| run.top == row) {
        for column in run.left..run.left + run.width {
            covers[usize::from(column)] = run.cover;
        }
    }
    covers
}

#[test]
fn takes_nine_by_eleven_points_at_any_scale() {
    assert_eq!(Silhouette::at(SCREEN).size(), [9.0, 11.0]);
    assert_eq!(Silhouette::at(RETINA).size(), [18.0, 22.0]);
    assert_eq!(Silhouette::at(1.6).size(), [15.0, 18.0]);
    assert!(Silhouette::at(1.6).is_at(1.6));
    assert!(!Silhouette::at(1.6).is_at(2.0));
    assert!(Silhouette::default().is_at(0.0));
}

#[test]
fn fills_the_head_and_smooths_its_edges() {
    let silhouette = Silhouette::at(SCREEN);

    let across_the_head = covers(&silhouette, 2);

    assert_eq!(across_the_head[3..6], [OPAQUE; 3]);
    assert!((FAINTEST..OPAQUE).contains(&across_the_head[2]));
    assert!((FAINTEST..OPAQUE).contains(&across_the_head[6]));
    assert_eq!(across_the_head[0..2], [0, 0]);
    assert_eq!(across_the_head[7..], [0, 0]);
}

#[test]
fn draws_the_bottom_of_the_shoulders_in_one_run() {
    let silhouette = Silhouette::at(SCREEN);

    let full: Vec<&Run> = silhouette
        .runs
        .iter()
        .filter(|run| run.top == 10 && run.cover == OPAQUE)
        .collect();

    assert_eq!(full.len(), 1);
    assert_eq!((full[0].left, full[0].width), (1, 7));
}

#[test]
fn leaves_the_neck_empty() {
    let silhouette = Silhouette::at(RETINA);

    assert!(covers(&silhouette, 10).iter().all(|&cover| cover == 0));
    assert!(covers(&silhouette, 9).iter().any(|&cover| cover > 0));
    assert!(covers(&silhouette, 12).iter().any(|&cover| cover > 0));
}

#[test]
fn is_the_same_on_both_sides() {
    let silhouette = Silhouette::at(RETINA);

    for row in 0..22 {
        let covers = covers(&silhouette, row);
        let mirrored: Vec<u8> = covers.iter().rev().copied().collect();
        let apart = covers
            .iter()
            .zip(&mirrored)
            .all(|(left, right)| left.abs_diff(*right) <= 1);
        assert!(apart, "row {row}: {covers:?}");
    }
}

#[test]
fn never_draws_a_pixel_it_barely_touches() {
    let silhouette = Silhouette::at(1.6);

    assert!(silhouette.runs.iter().all(|run| run.cover >= FAINTEST));
    assert!(silhouette.runs.iter().any(|run| run.cover < OPAQUE));
}

#[test]
fn draws_one_pixel_high_rows_from_its_rounded_corner_dimmed_by_what_they_cover() {
    let silhouette = Silhouette::at(SCREEN);
    let paint = Paint::ink(Ink::Full);

    let drawn: Vec<Rectangle> = silhouette.drawn([10.4, 20.6], paint).collect();

    assert_eq!(drawn.len(), silhouette.runs.len());
    for (rectangle, run) in drawn.iter().zip(&silhouette.runs) {
        assert_eq!(rectangle.left, 10.0 + f32::from(run.left));
        assert_eq!(rectangle.top, 21.0 + f32::from(run.top));
        assert_eq!(rectangle.width, f32::from(run.width));
        assert_eq!(rectangle.height, 1.0);
        assert_eq!(rectangle.color, paint.dimmed(run.cover));
    }
    assert!(drawn.iter().any(|rectangle| rectangle.color == paint));
    assert!(drawn.iter().any(|rectangle| rectangle.color.dim < OPAQUE));
}

#[test]
fn draws_nothing_without_a_scale() {
    assert!(
        Silhouette::at(0.0)
            .drawn([0.0, 0.0], Paint::ink(Ink::Full))
            .next()
            .is_none()
    );
    assert_eq!(Silhouette::default().size(), [0.0, 0.0]);
}
