use std::time::Duration;

use dot_api::ReplyEvent;
use dot_transcript::{Spot, TEXT_INDENT};
use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::flow::select::Selecting;
use crate::conversation::flow::select::outline::CONTOUR;
use crate::conversation::flow::select::tests::{point_at, replied, reply_row};
use crate::conversation::flow::tests::{RETINA, after, layout, scene_at, sent, stage};

const PARAGRAPHS: &str = "a first paragraph\n\nthe second one goes on   \nlast";

fn drawn_at(conversation: &Conversation, now: Instant) -> Drawn {
    drawn_in(conversation, (&stage(), &layout()), now)
}

fn drawn_in(
    conversation: &Conversation,
    (stage, layout): (&Stage, &Layout),
    now: Instant,
) -> Drawn {
    let top = stage.top_row(conversation.reader.focus(now));
    conversation.selection_drawn((stage, layout), top, now, &IdTranslator)
}

fn selected(now: Instant) -> Conversation {
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout(), now);
    conversation.select_drag(point_at(&conversation, row, 9, now), &layout(), now);
    conversation
}

fn with_paragraphs(start: Instant) -> (Conversation, usize) {
    let mut conversation = sent("question", start);
    conversation.hear(
        ReplyEvent::Text(PARAGRAPHS.to_owned()),
        start,
        &IdTranslator,
    );
    conversation.end_reply(None, Some(12), start);
    scene_at(&mut conversation, after(start, 5_000));
    let first = (0..conversation.transcript.total())
        .find(|&row| {
            conversation
                .transcript
                .row_text(row)
                .is_some_and(|shown| shown.text.starts_with("a first"))
        })
        .unwrap();
    (conversation, first)
}

fn made(conversation: &mut Conversation, [anchor, head]: [Spot; 2], now: Instant) {
    conversation.selecting = Some(Selecting {
        selection: Selection { anchor, head },
        made: Some(now),
        copied: None,
    });
}

fn x_at(column: usize) -> f32 {
    let advance = layout().advance();
    (stage().column[0] + (f32::from(TEXT_INDENT) + to_f32(rows(column))) * advance).round()
}

fn same(left: f32, right: f32) -> bool {
    (left - right).abs() < f32::EPSILON
}

fn spot(row: usize, column: usize) -> Spot {
    Spot { row, column }
}

#[test]
fn washes_the_text_being_selected_in_the_live_colour_at_thirty_eight_percent() {
    let now = after(Instant::now(), 5_000);
    let conversation = selected(now);

    let drawn = drawn_at(&conversation, now);

    assert_eq!(drawn.grounds.len(), 1);
    let wash = drawn.grounds[0];
    assert_eq!(
        wash.color,
        Paint::shaded(Ink::Live, 0, opacity(WASH_MAKING))
    );
    assert_eq!(wash.color.alpha, OPAQUE);
    assert_eq!(wash.left, x_at(2));
    assert_eq!(wash.width, x_at(9) - x_at(2));
    assert_eq!(drawn.panes.len(), 0);
}

#[test]
fn shows_a_live_i_beam_at_each_end_and_corners_around_the_words_while_dragging() {
    let now = after(Instant::now(), 5_000);
    let conversation = selected(now);

    let drawn = drawn_at(&conversation, now);

    let live: Vec<Rectangle> = drawn
        .marks
        .iter()
        .filter(|mark| mark.color == Paint::ink(Ink::Live))
        .copied()
        .collect();
    assert_eq!(live.len(), 2 * 3 + 16);
    let stems: Vec<f32> = live
        .iter()
        .filter(|mark| mark.width == HAIRLINE && mark.height > stage().points(BRACKET))
        .map(|mark| mark.left)
        .collect();
    assert_eq!(stems, [x_at(2), x_at(9)]);
    let words_end = x_at("reply 0 goes".len());
    assert!(
        live.iter()
            .any(|mark| same(mark.left, words_end) && same(mark.width, HAIRLINE))
    );
    assert!(live.iter().any(|mark| same(mark.left, x_at(0) - HAIRLINE)));
}

#[test]
fn gives_each_i_beam_two_serifs_inside_its_row() {
    let stage = stage();
    let [stem, top, bottom] = beam([100.0, 40.0], &stage, Ink::Live);

    let inset = stage.points(BEAM_INSET);
    let serif = stage.points(BEAM_SERIF);
    assert_eq!((stem.left, stem.top), (100.0, 40.0 + inset));
    assert_eq!(stem.top + stem.height, 40.0 + stage.row.round() - inset);
    assert_eq!(
        (top.left, top.top, top.width),
        (100.0 - serif, stem.top, 2.0 * serif + HAIRLINE)
    );
    assert_eq!(bottom.top + bottom.height, stem.top + stem.height);
    assert_eq!(bottom.height, HAIRLINE);
}

#[test]
fn settles_to_a_faint_white_drawn_opaque_with_cooling_corners() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let settled = drawn_at(&conversation, after(now, 500));

    assert_eq!(
        settled.grounds[0].color,
        Paint::shaded(Ink::Full, 0, opacity(WASH_MADE))
    );
    assert_eq!(settled.marks.len(), 16);
    assert!(
        settled
            .marks
            .iter()
            .all(|mark| mark.color.ink == MADE_MARK_INK)
    );
    assert_eq!(settled.panes.len(), 1);
}

#[test]
fn warms_the_wash_halfway_through_settling() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let halfway = drawn_at(&conversation, after(now, 100)).grounds[0].color;

    assert!(halfway.heat > 0 && halfway.heat < u8::MAX);
    assert!(halfway.dim < opacity(WASH_MAKING) && halfway.dim > opacity(WASH_MADE));
    assert_eq!(halfway.alpha, OPAQUE);
}

#[test]
fn cools_the_corners_from_the_live_colour_to_sixty_four_percent_and_warms_them_on_copy() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);
    let heat_at = |conversation: &Conversation, at: Instant| {
        let drawn = drawn_at(conversation, at);
        assert_ne!(drawn.marks.len(), 0);
        drawn.marks[0].color.heat
    };

    let halfway = heat_at(&conversation, after(now, 300));
    assert!(halfway > 0 && halfway < u8::MAX, "{halfway}");
    assert_eq!(heat_at(&conversation, after(now, 1_300)), 0);
    conversation.mark_copied(after(now, 2_000));
    assert!(heat_at(&conversation, after(now, 2_100)) > 0);
    assert!(conversation.selection_moves(after(now, 2_100)));
}

#[test]
fn ends_each_row_where_its_text_ends_and_marks_a_blank_row_with_a_stub() {
    let now = after(Instant::now(), 5_000);
    let (mut conversation, first) = with_paragraphs(now);
    made(&mut conversation, [spot(first, 2), spot(first + 3, 2)], now);

    let washes = drawn_at(&conversation, after(now, 5_000)).grounds;

    let reaches: Vec<[f32; 2]> = washes
        .iter()
        .map(|wash| [wash.left, wash.left + wash.width])
        .collect();
    let half = (stage().column[0] + (f32::from(TEXT_INDENT) + 0.5) * layout().advance()).round();
    assert_eq!(
        reaches,
        [
            [x_at(2), x_at("a first paragraph".len())],
            [x_at(0), half],
            [x_at(0), x_at("the second one goes on".len())],
            [x_at(0), x_at(2)],
        ]
    );
    let tops: Vec<f32> = washes.iter().map(|wash| wash.top).collect();
    assert!(
        washes
            .windows(2)
            .all(|pair| same(pair[0].top + pair[0].height, pair[1].top)),
        "{tops:?}"
    );
}

#[test]
fn stops_at_the_row_before_an_end_at_its_very_start() {
    let now = after(Instant::now(), 5_000);
    let (mut conversation, first) = with_paragraphs(now);
    made(&mut conversation, [spot(first, 2), spot(first + 2, 0)], now);

    let drawn = drawn_at(&conversation, after(now, 5_000));

    assert_eq!(drawn.grounds.len(), 2);
    let bottom = drawn.grounds[1].top + drawn.grounds[1].height;
    let corners_below = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.dim == OPAQUE && same(mark.top + mark.height, bottom + HAIRLINE));
    assert_eq!(corners_below.count(), 4);
}

#[test]
fn draws_a_faint_contour_hugging_the_rows_of_a_selection_over_several() {
    let now = after(Instant::now(), 5_000);
    let (mut conversation, first) = with_paragraphs(now);
    made(&mut conversation, [spot(first, 2), spot(first + 3, 2)], now);

    let drawn = drawn_at(&conversation, after(now, 5_000));

    let contour: Vec<&Rectangle> = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.dim == opacity(CONTOUR))
        .collect();
    assert_ne!(contour.len(), 0);
    let second = &drawn.grounds[2];
    let right_of_the_second = x_at("the second one goes on".len());
    assert!(contour.iter().any(|mark| {
        same(mark.left, right_of_the_second)
            && same(mark.top, second.top)
            && same(mark.height, second.height)
    }));
    let solid = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.dim == OPAQUE)
        .count();
    assert_eq!(solid, 16);
    assert!(
        drawn
            .marks
            .iter()
            .any(|mark| same(mark.left, right_of_the_second - HAIRLINE)
                && mark.color.dim != OPAQUE
                && mark.color.dim > opacity(CONTOUR))
    );
}

#[test]
fn draws_no_corner_on_a_row_the_window_cuts() {
    let now = after(Instant::now(), 5_000);
    let (conversation, first) = with_paragraphs(now);
    let selection = Selection {
        anchor: spot(first, 2),
        head: spot(first + 3, 2),
    };
    let stage = stage();
    let layout = layout();
    let place = |top_row: f64| (&stage, &layout, top_row);

    let whole = conversation.outline_of(&selection, place(rows(first) - 2.0));
    let cut_above = conversation.outline_of(&selection, place(rows(first) + 1.5));
    let rows_shown = f64::from(stage.floor / stage.row);
    let cut_below = conversation.outline_of(&selection, place(rows(first) + 2.5 - rows_shown));

    assert_eq!(
        (whole.spans.len(), whole.opens, whole.closes),
        (4, true, true)
    );
    assert_eq!(
        (cut_above.spans.len(), cut_above.opens, cut_above.closes),
        (3, false, true)
    );
    assert_eq!((cut_below.opens, cut_below.closes), (true, false));
    assert!(cut_below.spans.len() < 4);
}

#[test]
fn clips_what_it_draws_to_the_conversation_above_the_status_bar() {
    let stage = stage();
    let rectangle = |top: f32, height: f32| Rectangle {
        left: 0.0,
        top,
        width: 4.0,
        height,
        color: Paint::ink(Ink::Full),
    };
    let drawn = Drawn {
        grounds: vec![rectangle(-10.0, 20.0), rectangle(stage.floor - 5.0, 20.0)],
        marks: vec![rectangle(stage.floor, 1.0), rectangle(-1.0, 1.0)],
        panes: Vec::new(),
    };

    let kept = clipped(drawn, &stage);

    let spans: Vec<[f32; 2]> = kept
        .grounds
        .iter()
        .map(|ground| [ground.top, ground.height])
        .collect();
    assert_eq!(spans, [[0.0, 10.0], [stage.floor - 5.0, 5.0]]);
    assert_eq!(kept.marks.len(), 0);
}

#[test]
fn leaves_the_size_out_of_a_window_too_narrow_for_it() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);
    let narrow = Layout::reflowed(PhysicalSize::new(1_000, 1_400), RETINA);

    let drawn = drawn_in(
        &conversation,
        (&Stage::of(&narrow), &narrow),
        after(now, 500),
    );

    assert_eq!(drawn.panes.len(), 0);
    assert_ne!(drawn.grounds.len(), 0);
}

#[test]
fn shows_a_grey_i_beam_where_the_pointer_would_start_a_selection() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.point(Some(point_at(&conversation, row, 3, now)));

    let drawn = drawn_at(&conversation, now);

    assert_eq!(drawn.marks.len(), 3);
    assert!(drawn.marks.iter().all(|mark| mark.color.ink == BEAM_INK));
    assert_eq!(drawn.marks[0].left, x_at(3));
    conversation.point(Some([1.0, 1.0]));
    assert_eq!(drawn_at(&conversation, now).marks.len(), 0);
    conversation.point(None);
    assert_eq!(drawn_at(&conversation, now).marks.len(), 0);
}

#[test]
fn keeps_the_grey_i_beam_over_text_once_a_selection_is_made_but_not_while_dragging() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    let row = reply_row(&conversation);
    conversation.point(Some(point_at(&conversation, row, 14, now)));
    let grey = |drawn: &Drawn| {
        drawn
            .marks
            .iter()
            .filter(|mark| mark.color.ink == BEAM_INK)
            .count()
    };

    assert_eq!(grey(&drawn_at(&conversation, now)), 0);
    conversation.select_release(now);
    let later = now + Duration::from_secs(5);
    let drawn = drawn_at(&conversation, later);
    assert_eq!(grey(&drawn) - 16, 3);
}
