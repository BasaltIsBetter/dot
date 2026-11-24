use dot_api::Hue;

use super::*;
use crate::sessions::SessionKey;
use crate::sessions::identity::NameField;
use crate::sessions::rail::tests::{card, centre_of, rail_of, texts};
use crate::sessions::{Foot, State};

fn coloured(hue: Hue) -> Card {
    Card {
        hue: Some(hue),
        ..card("Dot", true, State::Idle, Foot::EMPTY)
    }
}

fn marks_in(rail: &Rail, rgb: [u8; 3]) -> Vec<Rectangle> {
    rail.marks
        .iter()
        .filter(|mark| mark.color.ink == Ink::Rgb(rgb))
        .copied()
        .collect()
}

#[test]
fn puts_a_square_of_the_session_s_colour_before_its_name() {
    let rail = rail_of(&[coloured(Hue::Violet)], None);

    let squares: Vec<Rectangle> = marks_in(&rail, Hue::Violet.rgb())
        .into_iter()
        .filter(|mark| (mark.width - mark.height).abs() < f32::EPSILON && mark.width > 2.0)
        .collect();
    assert_eq!(squares.len(), 1);
    let name = rail
        .panes
        .iter()
        .find(|pane| pane.lines[0].line.spans[0].text == "Dot")
        .unwrap();
    assert!(name.left > squares[0].left + squares[0].width);
    let square_centre = [squares[0].left + 1.0, squares[0].top + 1.0];
    assert_eq!(rail.hit(square_centre), Some(Hit::Hue(0)));
}

#[test]
fn draws_no_square_without_session_colours() {
    let rail = rail_of(&[card("Dot", true, State::Idle, Foot::EMPTY)], None);

    assert!(!rail.hits.iter().any(|(_, hit)| matches!(hit, Hit::Hue(_))));
}

#[test]
fn lets_a_click_on_the_name_rename_the_session() {
    let rail = rail_of(&[card("Dot", true, State::Idle, Foot::EMPTY)], None);

    let name = centre_of(&rail, Hit::Name(0));

    assert_eq!(rail.hit(name), Some(Hit::Name(0)));
}

#[test]
fn underlines_the_name_under_the_pointer() {
    let cards = [card("Dot", true, State::Idle, Foot::EMPTY)];
    let resting = rail_of(&cards, None);
    let name = centre_of(&resting, Hit::Name(0));

    let hovered = rail_of(&cards, Some(name));

    let underlined = |rail: &Rail| {
        rail.marks
            .iter()
            .filter(|mark| mark.color.ink == Ink::Receding && mark.height <= HAIRLINE)
            .count()
    };
    assert_eq!(underlined(&hovered), underlined(&resting) + 1);
}

#[test]
fn draws_a_new_name_in_its_cooling_tint() {
    let tinted = Card {
        name_tint: Some([10, 20, 30]),
        ..card("api", false, State::Idle, Foot::EMPTY)
    };

    let rail = rail_of(&[tinted], None);

    let name = rail
        .panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .flat_map(|placed| &placed.line.spans)
        .find(|span| span.text == "api")
        .unwrap();
    assert_eq!(name.ink, Ink::Rgb([10, 20, 30]));
}

fn renaming(text: &str, whole: bool, hue: Option<Hue>) -> Card {
    Card {
        field: Some(NameField {
            key: SessionKey(1),
            text: text.to_owned(),
            caret: 1,
            whole,
        }),
        hue,
        ..card("Dot", true, State::Idle, Foot::EMPTY)
    }
}

#[test]
fn shows_the_name_being_typed_with_its_colour_washed_behind_when_whole() {
    let rail = rail_of(&[renaming("api", true, Some(Hue::Cyan))], None);

    assert!(texts(&rail).contains(&"api".to_owned()));
    let washed = marks_in(&rail, Hue::Cyan.rgb())
        .into_iter()
        .filter(|mark| mark.color.alpha == WASH)
        .count();
    assert_eq!(washed, 1);
    assert!(rail.hits.iter().any(|(_, hit)| *hit == Hit::Name(0)));
}

#[test]
fn draws_a_caret_and_an_underline_in_the_live_colour_without_session_colours() {
    let rail = rail_of(&[renaming("api", false, None)], None);

    let live: Vec<Rectangle> = rail
        .marks
        .iter()
        .filter(|mark| mark.color.ink == Ink::Live)
        .copied()
        .collect();
    assert_eq!(live.len(), 2);
    assert!(live.iter().any(|mark| mark.width <= HAIRLINE));
    assert!(live.iter().any(|mark| mark.height <= HAIRLINE));
    assert!(!rail.marks.iter().any(|mark| mark.color.alpha == WASH));
}
