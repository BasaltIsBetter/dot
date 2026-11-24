use crate::canvas::TextSize;
use crate::canvas::{Paint, Rectangle};
use crate::conversation::Stage;
use crate::sessions::card::Card;
use crate::sessions::rail::Hit;
use crate::sessions::rail::Rail;
use crate::sessions::rail::tests::{
    RAILED, card, centre_of, layout, pane_of, rail_of, texts, two_cards,
};
use crate::sessions::{Foot, State};
use dot_transcript::Ink;

#[test]
fn offers_to_close_only_the_card_under_the_pointer_and_never_the_last_session() {
    let unhovered = rail_of(&two_cards(), None);
    let over_the_second = centre_of(&unhovered, Hit::Switch(1));

    let hovered = rail_of(&two_cards(), Some(over_the_second));
    let alone = rail_of(
        &two_cards()[..1],
        Some(centre_of(&unhovered, Hit::Switch(0))),
    );

    assert!(
        !unhovered
            .hits
            .iter()
            .any(|(_, hit)| matches!(hit, Hit::Close(_)))
    );
    assert!(hovered.hits.iter().any(|(_, hit)| *hit == Hit::Close(1)));
    assert!(texts(&hovered).contains(&"×".to_owned()));
    assert!(
        !alone
            .hits
            .iter()
            .any(|(_, hit)| matches!(hit, Hit::Close(_)))
    );
}

#[test]
fn writes_each_name_with_its_glyph_and_the_rate_of_its_last_reply() {
    let rail = rail_of(&two_cards(), None);

    let texts = texts(&rail);
    assert!(texts.contains(&"Dot".to_owned()));
    assert!(texts.contains(&"◉".to_owned()));
    assert!(texts.contains(&"●".to_owned()));
    assert!(texts.contains(&"58 unit-tokens-per-second".to_owned()));
    assert!(texts.contains(&"––– unit-tokens-per-second".to_owned()));
}

#[test]
fn a_card_without_a_reply_shows_dashes_for_its_rate_and_an_unlit_spark() {
    let cards = [card("Dot", true, State::Idle, Foot::EMPTY)];

    let rail = rail_of(&cards, None);

    assert_eq!(texts(&rail), ["Dot", "◉", "––– unit-tokens-per-second"]);
    let count = |ink| {
        rail.marks
            .iter()
            .filter(|mark| mark.color == Paint::ink(ink))
            .count()
    };
    assert_eq!(count(Ink::Faint), 16);
    assert_eq!(count(Ink::Trace), 32);
    assert_eq!(count(Ink::Live), 0);
}

fn foot_top_of(rail: &Rail, stage: &Stage, above: f32) -> f32 {
    let ([_, top, _, _], _) = rail.hits[0];
    top + stage.points(9.0) + stage.points(above)
}

#[test]
fn centres_the_spark_in_the_foot_row_and_ends_the_rate_at_the_inner_edge() {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    let rail = rail_of(&[card("Dot", true, State::Idle, Foot::EMPTY)], None);
    let [left, right] = stage.rail.unwrap();
    let foot_top = foot_top_of(&rail, &stage, 22.0);

    let spark: Vec<&Rectangle> = rail
        .marks
        .iter()
        .filter(|mark| mark.color == Paint::ink(Ink::Faint) || mark.color == Paint::ink(Ink::Trace))
        .collect();
    let spark_top = spark.iter().map(|mark| mark.top).fold(f32::MAX, f32::min);
    let spark_bottom = spark
        .iter()
        .map(|mark| mark.top + mark.height)
        .fold(f32::MIN, f32::max);
    let spark_left = spark.iter().map(|mark| mark.left).fold(f32::MAX, f32::min);
    let expected_top = (foot_top + (stage.points(12.0) - stage.points(10.0)) / 2.0).round();
    assert_eq!(spark_top, expected_top);
    assert!((spark_bottom - (expected_top + stage.points(10.0))).abs() <= 1.0);
    assert!((spark_left - (left + stage.points(13.0))).abs() <= 1.0);

    let text = "––– unit-tokens-per-second";
    let rate = pane_of(&rail, text);
    let width = 26.0 * layout.advance_of(TextSize::Tiny);
    assert_eq!(text.chars().count(), 26);
    assert!((rate.left + width - (right - stage.points(13.0))).abs() < 0.01);
    assert_eq!(
        rate.lines[0].top,
        layout.text_top(TextSize::Tiny, foot_top, stage.points(12.0))
    );
}

#[test]
fn writes_the_rate_live_only_while_claude_writes() {
    let ink_of = |state| {
        let rail = rail_of(&[card("Dot", false, state, Foot::EMPTY)], None);
        pane_of(&rail, "––– unit-tokens-per-second").lines[0]
            .line
            .spans[0]
            .ink
    };

    assert_eq!(ink_of(State::Writing), Ink::Live);
    assert_eq!(ink_of(State::Idle), Ink::Secondary);
    assert_eq!(ink_of(State::Paused), Ink::Secondary);
}

#[test]
fn moves_the_foot_under_an_open_palette() {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    let picking = Card {
        picking: true,
        ..card("Dot", true, State::Idle, Foot::EMPTY)
    };

    let rail = rail_of(&[picking], None);

    let rate = pane_of(&rail, "––– unit-tokens-per-second");
    let foot_top = foot_top_of(&rail, &stage, 16.0 + 6.0 + 4.0 + 16.0);
    let palette_top = foot_top_of(&rail, &stage, 16.0 + 4.0);
    let squares_top = rail
        .hits
        .iter()
        .filter(|(_, hit)| matches!(hit, Hit::Pick(..)))
        .map(|([_, top, _, _], _)| *top)
        .fold(f32::MAX, f32::min);
    assert!(squares_top >= palette_top - 0.01);
    assert!(squares_top < palette_top + stage.points(16.0));
    assert_eq!(
        rate.lines[0].top,
        layout.text_top(TextSize::Tiny, foot_top, stage.points(12.0))
    );
}
