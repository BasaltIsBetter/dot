use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::canvas::TextSize;
use crate::sessions::{Foot, State};
use dot_transcript::SPARK_COLUMNS;

pub(super) const RAILED: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);
const NARROW: PhysicalSize<u32> = PhysicalSize::new(1000, 700);

pub(super) fn layout(size: PhysicalSize<u32>) -> Layout {
    Layout::reflowed(size, 2.0)
}

fn foot(rate: Option<u64>) -> Foot {
    Foot {
        rate,
        rates: [0; SPARK_COLUMNS],
    }
}

pub(super) fn card(name: &str, here: bool, state: State, foot: Foot) -> Card {
    Card {
        name: name.to_owned(),
        here,
        state,
        foot,
        hue: None,
        picking: false,
        field: None,
        name_tint: None,
    }
}

pub(super) fn two_cards() -> Vec<Card> {
    vec![
        card("Dot", true, State::Idle, foot(Some(58))),
        card("session 2", false, State::Writing, foot(None)),
    ]
}

pub(super) fn rail_of(cards: &[Card], hover: Option<[f32; 2]>) -> Rail {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    Rail::of(
        cards,
        (&stage, &layout),
        hover,
        Instant::now(),
        &IdTranslator,
    )
    .unwrap()
}

pub(super) fn texts(rail: &Rail) -> Vec<String> {
    rail.panes
        .iter()
        .flat_map(|pane| &pane.lines)
        .map(|placed| {
            placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect()
        })
        .collect()
}

pub(super) fn centre_of(rail: &Rail, wanted: Hit) -> [f32; 2] {
    let ([left, top, width, height], _) = rail.hits.iter().find(|(_, hit)| *hit == wanted).unwrap();
    [left + width / 2.0, top + height / 2.0]
}

#[test]
fn has_no_room_in_a_narrow_window() {
    let layout = layout(NARROW);
    let stage = Stage::of(&layout);

    let rail = Rail::of(
        &two_cards(),
        (&stage, &layout),
        None,
        Instant::now(),
        &IdTranslator,
    );

    assert!(rail.is_none());
}

#[test]
fn stacks_the_cards_down_the_middle_of_the_rail_then_the_new_one() {
    let rail = rail_of(&two_cards(), None);
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);

    let hits: Vec<Hit> = rail.hits.iter().map(|(_, hit)| *hit).collect();
    assert_eq!(
        hits,
        [
            Hit::Switch(0),
            Hit::Name(0),
            Hit::Switch(1),
            Hit::Name(1),
            Hit::Open
        ]
    );
    let cards: Vec<[f32; 4]> = rail
        .hits
        .iter()
        .filter(|(_, hit)| matches!(hit, Hit::Switch(_) | Hit::Open))
        .map(|(bounds, _)| *bounds)
        .collect();
    let tops: Vec<f32> = cards.iter().map(|[_, top, _, _]| *top).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]));
    let [_, first, _, _] = cards[0];
    let [_, last, _, height] = cards[2];
    let room_top = stage.points(24.0);
    let room_bottom = stage.height - stage.points(64.0);
    assert!(((first - room_top) - (room_bottom - (last + height))).abs() <= 2.0);
}

#[test]
fn gives_every_card_its_foot_across_the_rail_even_before_a_reply() {
    let stage = Stage::of(&layout(RAILED));
    let cards = vec![
        card("Dot", true, State::Idle, foot(Some(58))),
        card("session 2", false, State::Idle, Foot::EMPTY),
    ];

    let rail = rail_of(&cards, None);

    let bounds: Vec<[f32; 4]> = rail
        .hits
        .iter()
        .filter(|(_, hit)| matches!(hit, Hit::Switch(_) | Hit::Open))
        .map(|(bounds, _)| *bounds)
        .collect();
    let [left, right] = stage.rail.unwrap();
    assert!(
        bounds
            .iter()
            .all(|[at, _, width, _]| (*at - left).abs() < f32::EPSILON
                && (*width - (right - left)).abs() < f32::EPSILON)
    );
    let heights: Vec<f32> = bounds.iter().map(|[_, _, _, height]| *height).collect();
    assert_eq!(
        heights,
        [stage.points(52.0), stage.points(52.0), stage.points(30.0)]
    );
}

#[test]
fn finds_a_card_only_inside_its_edges() {
    let rail = rail_of(&two_cards(), None);
    let ([left, top, width, height], _) = rail.hits[0];
    let middle = [left + width / 2.0, top + height / 2.0];
    let ([_, last_top, _, last_height], _) = rail.hits[2];

    assert_eq!(rail.hit([left, top]), Some(Hit::Switch(0)));
    assert_eq!(rail.hit([left + width, middle[1]]), None);
    assert_eq!(rail.hit([middle[0], top + height]), None);
    assert_eq!(rail.hit([left + width + 10.0, middle[1]]), None);
    assert_eq!(rail.hit([middle[0], last_top + last_height + 10.0]), None);
}

fn crosses(cards: &[Card], hover: [f32; 2]) -> usize {
    rail_of(cards, Some(hover))
        .hits
        .iter()
        .filter(|(_, hit)| matches!(hit, Hit::Close(_)))
        .count()
}

#[test]
fn offers_a_cross_only_while_the_pointer_is_inside_a_card() {
    let cards = two_cards();
    let ([left, top, width, height], _) = rail_of(&cards, None).hits[0];
    let middle = [left + width / 2.0, top + height / 2.0];

    assert_eq!(crosses(&cards, middle), 1);
    assert_eq!(crosses(&cards, [left + width, middle[1]]), 0);
    assert_eq!(crosses(&cards, [left - 1.0, middle[1]]), 0);
    assert_eq!(crosses(&cards, [left + width + 10.0, middle[1]]), 0);
    assert_eq!(crosses(&cards, [middle[0], top - 1.0]), 0);
    assert_eq!(crosses(&cards, [middle[0], top + height]), 0);
}

#[test]
fn never_offers_to_close_the_only_session() {
    let lone = [card("Dot", true, State::Idle, Foot::EMPTY)];
    let ([left, top, width, height], _) = rail_of(&lone, None).hits[0];

    assert_eq!(crosses(&lone, [left + width / 2.0, top + height / 2.0]), 0);
}

pub(super) fn pane_of<'a>(rail: &'a Rail, text: &str) -> &'a Pane {
    rail.panes
        .iter()
        .find(|pane| {
            pane.lines.iter().any(|placed| {
                placed
                    .line
                    .spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
                    == text
            })
        })
        .unwrap()
}

#[test]
fn writes_inside_the_card_and_its_foot_under_its_name() {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    let rail = rail_of(&two_cards(), None);
    let ([left, top, _, _], _) = rail.hits[0];

    let name = pane_of(&rail, "Dot");
    let foot = pane_of(&rail, "58 unit-tokens-per-second");

    assert_eq!(name.left, left + stage.points(13.0));
    let foot_top = top + stage.points(9.0) + stage.points(22.0);
    assert_eq!(
        foot.lines[0].top,
        layout.text_top(TextSize::Tiny, foot_top, stage.points(12.0))
    );
}

#[test]
fn finds_the_card_under_a_click() {
    let rail = rail_of(&two_cards(), None);

    assert_eq!(
        rail.hit(centre_of(&rail, Hit::Switch(1))),
        Some(Hit::Switch(1))
    );
    assert_eq!(rail.hit(centre_of(&rail, Hit::Open)), Some(Hit::Open));
    assert_eq!(rail.hit([0.0, 0.0]), None);
}

fn many_cards(count: usize, here: usize) -> Vec<Card> {
    (0..count)
        .map(|index| card(&format!("s{index}"), index == here, State::Idle, foot(None)))
        .collect()
}

#[test]
fn keeps_every_card_of_a_long_column_above_the_status_bar_with_the_one_here_and_the_new_one() {
    let layout = layout(RAILED);
    let stage = Stage::of(&layout);
    let room_bottom = stage.height - stage.points(64.0);

    for here in [0, 20, 39] {
        let rail = rail_of(&many_cards(40, here), None);

        let hits: Vec<Hit> = rail.hits.iter().map(|(_, hit)| *hit).collect();
        assert!(hits.contains(&Hit::Switch(here)), "{here}: {hits:?}");
        assert_eq!(hits.last(), Some(&Hit::Open));
        assert!(hits.len() < 41);
        assert!(
            rail.hits
                .iter()
                .all(|([_, top, _, height], _)| top + height <= room_bottom + 1.0)
        );
    }
}

#[test]
fn shows_the_cards_around_the_one_here_in_their_order() {
    assert_eq!(in_sight(&[10.0; 10], 2.0, 36.0, 5), 3..6);
    assert_eq!(in_sight(&[10.0; 10], 2.0, 36.0, 0), 0..3);
    assert_eq!(in_sight(&[10.0; 10], 2.0, 1_000.0, 4), 0..10);
    assert_eq!(in_sight(&[10.0; 3], 2.0, 5.0, 1), 1..2);
    assert_eq!(in_sight(&[], 2.0, 100.0, 0), 0..0);
}
