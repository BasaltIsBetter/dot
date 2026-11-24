use dot_transcript::TEXT_INDENT;

use super::*;
use crate::conversation::flow::tests::{after, answered, layout, scene_at, stage};

pub(super) fn reply_row(conversation: &Conversation) -> usize {
    (0..conversation.transcript.total())
        .find(|&row| {
            conversation
                .transcript
                .row_text(row)
                .is_some_and(|shown| shown.text.starts_with("reply 0"))
        })
        .unwrap()
}

pub(super) fn point_at(
    conversation: &Conversation,
    row: usize,
    column: usize,
    now: Instant,
) -> [f32; 2] {
    let stage = stage();
    let layout = layout();
    let top = stage.row_top(row, stage.top_row(conversation.reader.focus(now)));
    let left = stage.column[0] + f32::from(TEXT_INDENT) * layout.advance();
    [
        left + to_f32(rows(column)) * layout.advance() + 1.0,
        top + stage.row / 2.0,
    ]
}

pub(super) fn replied(now: Instant) -> Conversation {
    let mut conversation = answered(1, now);
    scene_at(&mut conversation, after(now, 5_000));
    conversation
}

#[test]
fn a_press_on_text_starts_a_selection_and_a_press_beside_it_clears_it() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);

    assert!(conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout(), now));
    assert!(conversation.is_dragging());
    assert!(!conversation.select_press([1.0, 1.0], 1, &layout(), now));
    assert!(conversation.selecting.is_none());
}

#[test]
fn dragging_then_releasing_selects_whole_words() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout(), now);

    conversation.select_drag(point_at(&conversation, row, 9, now), &layout(), now);
    assert_eq!(conversation.selected_text(), None);
    conversation.select_release(now);

    assert_eq!(
        conversation.selected_text().as_deref(),
        Some("reply 0 goes")
    );
    assert!(!conversation.is_dragging());
}

#[test]
fn a_release_without_a_drag_selects_nothing() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout(), now);

    conversation.select_release(now);

    assert!(conversation.selecting.is_none());
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_the_paragraph() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    let on_goes = point_at(&conversation, row, 9, now);

    conversation.select_press(on_goes, 2, &layout(), now);
    assert_eq!(conversation.selected_text().as_deref(), Some("goes"));
    conversation.select_press(on_goes, 3, &layout(), now);
    assert_eq!(
        conversation.selected_text().as_deref(),
        Some("reply 0 goes on for a while")
    );
}

#[test]
fn clears_the_selection_and_says_whether_there_was_one() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 9, now), 2, &layout(), now);

    assert!(conversation.clear_selection());
    assert!(!conversation.clear_selection());
}

#[test]
fn knows_when_the_pointer_is_over_text() {
    let now = after(Instant::now(), 5_000);
    let conversation = replied(Instant::now());
    let row = reply_row(&conversation);

    assert!(conversation.is_over_text(point_at(&conversation, row, 3, now), &layout(), now));
    assert!(!conversation.is_over_text([1.0, 1.0], &layout(), now));
    assert!(!conversation.is_over_text(point_at(&conversation, row, 200, now), &layout(), now));
}

#[test]
fn keeps_moving_while_dragging_settling_and_after_a_copy() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    assert!(!conversation.selection_moves(now));
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout(), now);
    assert!(conversation.selection_moves(now));
    conversation.select_drag(point_at(&conversation, row, 9, now), &layout(), now);
    conversation.select_release(now);

    assert!(conversation.selection_moves(after(now, 1_100)));
    assert!(!conversation.selection_moves(after(now, 1_300)));
    conversation.mark_copied(after(now, 1_300));
    assert!(conversation.selection_moves(after(now, 3_600)));
    assert!(!conversation.selection_moves(after(now, 3_800)));
}

#[test]
fn scrolls_when_the_pointer_drags_near_the_top_or_the_bottom() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = answered(8, Instant::now());
    scene_at(&mut conversation, now);
    let row = reply_row(&conversation);
    let layout = layout();
    conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout, now);
    let before = conversation.reader.focus(after(now, 2_000));

    assert!(conversation.is_dragging());
    let later = after(now, 2_000);

    conversation.point(Some([200.0, 2.0]));
    for _ in 0..4 {
        scene_at(&mut conversation, now);
    }
    let up = conversation.reader.focus(later);
    conversation.point(Some([200.0, stage().floor - 2.0]));
    for _ in 0..8 {
        scene_at(&mut conversation, now);
    }
    let down = conversation.reader.focus(later);
    conversation.point(Some([200.0, stage().floor / 2.0]));
    for _ in 0..4 {
        scene_at(&mut conversation, now);
    }
    let still = conversation.reader.focus(later);

    assert!(up < before, "{up} {before}");
    assert!(down > up, "{down} {up}");
    assert!((still - down).abs() < f64::EPSILON, "{still} {down}");
}

#[test]
fn never_selects_over_a_question_or_an_agent() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    let spot = point_at(&conversation, row, 2, now);
    conversation.agent_view = Some(crate::conversation::agent::AgentView::open(
        "a".to_owned(),
        now,
    ));

    assert!(!conversation.select_press(spot, 1, &layout(), now));
    assert!(!conversation.is_over_text(spot, &layout(), now));
}

#[test]
fn finds_the_column_nearest_to_a_point() {
    assert_eq!(column_at(10.0, 10.0, 5.0, "abc"), 0);
    assert_eq!(column_at(17.6, 10.0, 5.0, "abc"), 2);
    assert_eq!(column_at(99.0, 10.0, 5.0, "abc"), 3);
    assert_eq!(column_at(0.0, 10.0, 5.0, "abc"), 0);
}
