use dot_api::Hue;

use super::*;
use crate::sessions::rail::tests::{card, centre_of, rail_of};
use crate::sessions::{Foot, State};

fn picking(hue: Hue) -> Card {
    Card {
        hue: Some(hue),
        picking: true,
        ..card("Dot", true, State::Idle, Foot::EMPTY)
    }
}

#[test]
fn opens_eight_colours_under_the_name() {
    let rail = rail_of(&[picking(Hue::Blue)], None);

    let picks: Vec<Hue> = rail
        .hits
        .iter()
        .filter_map(|(_, hit)| match hit {
            Hit::Pick(0, hue) => Some(*hue),
            _ => None,
        })
        .collect();
    assert_eq!(picks, Hue::ALL);
}

#[test]
fn shows_the_current_colour_whole_and_ringed_and_the_others_dimmed() {
    let rail = rail_of(&[picking(Hue::Blue)], None);

    let swatch = |hue: Hue| {
        rail.marks
            .iter()
            .find(|mark| {
                mark.color.ink == Ink::Rgb(hue.rgb()) && mark.width > 2.0 && mark.width < 30.0
            })
            .copied()
            .unwrap()
    };
    assert_eq!(swatch(Hue::Blue).color.alpha, u8::MAX);
    assert_eq!(swatch(Hue::Red).color.alpha, RESTING);
    let ring = rail
        .marks
        .iter()
        .filter(|mark| mark.color.ink == Ink::Full)
        .count();
    assert_eq!(ring, 4);
}

#[test]
fn brightens_the_colour_under_the_pointer() {
    let resting = rail_of(&[picking(Hue::Blue)], None);
    let red = centre_of(&resting, Hit::Pick(0, Hue::Red));

    let hovered = rail_of(&[picking(Hue::Blue)], Some(red));

    let red_alpha = hovered
        .marks
        .iter()
        .find(|mark| mark.color.ink == Ink::Rgb(Hue::Red.rgb()))
        .unwrap()
        .color
        .alpha;
    assert_eq!(red_alpha, HOVERED);
    assert_eq!(hovered.hit(red), Some(Hit::Pick(0, Hue::Red)));
}

#[test]
fn makes_the_card_taller_while_the_colours_are_open() {
    let closed = card("Dot", true, State::Idle, Foot::EMPTY);
    let open = Card {
        picking: true,
        ..closed.clone()
    };
    let layout = crate::sessions::rail::tests::layout(crate::sessions::rail::tests::RAILED);
    let stage = crate::conversation::Stage::of(&layout);
    let frame = Frame {
        stage: &stage,
        layout: &layout,
        left: 0.0,
        right: 100.0,
        hover: None,
        now: std::time::Instant::now(),
    };

    let grown = frame.height_of(&open) - frame.height_of(&closed);

    let palette = crate::sessions::rail::PALETTE_GAP + PALETTE_ROW;
    assert!((grown - stage.points(palette)).abs() < 0.01);
}
