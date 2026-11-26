use super::*;
use crate::conversation::Stage;
use crate::sessions::SessionKey;
use crate::sessions::identity::NameField;
use crate::sessions::rail::PADDING_WIDE;
use crate::sessions::rail::tests::{OWN, RAILED, card, centre_of, layout, rail_of, texts};
use crate::sessions::{Foot, State};

#[test]
fn starts_the_name_at_the_inner_edge_of_the_card() {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    let rail = rail_of(&[card("Dot", true, State::Idle, Foot::EMPTY)], None);

    let name = rail
        .panes
        .iter()
        .find(|pane| pane.lines[0].line.spans[0].text == "Dot")
        .unwrap();
    let [left, _] = stage.rail.unwrap();
    assert!((name.left - (left + stage.points(PADDING_WIDE))).abs() < 0.01);
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

fn renaming(text: &str, whole: bool) -> Card {
    Card {
        field: Some(NameField {
            key: SessionKey(1),
            text: text.to_owned(),
            caret: 1,
            whole,
        }),
        ..card("Dot", false, State::Idle, Foot::EMPTY)
    }
}

#[test]
fn shows_the_name_being_typed_with_the_live_colour_washed_behind_when_whole() {
    let rail = rail_of(&[renaming("api", true)], None);

    assert!(texts(&rail).contains(&"api".to_owned()));
    let washed = rail
        .marks
        .iter()
        .filter(|mark| mark.color.ink == OWN && mark.color.dim == WASH)
        .count();
    assert_eq!(washed, 1);
    assert!(rail.hits.iter().any(|(_, hit)| *hit == Hit::Name(0)));
}

#[test]
fn draws_a_caret_and_an_underline_in_the_live_colour() {
    let rail = rail_of(&[renaming("api", false)], None);

    let live: Vec<Rectangle> = rail
        .marks
        .iter()
        .filter(|mark| mark.color.ink == OWN)
        .copied()
        .collect();
    assert_eq!(live.len(), 2);
    assert!(live.iter().any(|mark| mark.width <= HAIRLINE));
    assert!(live.iter().any(|mark| mark.height <= HAIRLINE));
    assert!(!rail.marks.iter().any(|mark| mark.color.dim == WASH));
}
