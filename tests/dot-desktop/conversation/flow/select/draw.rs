use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::flow::select::tests::{point_at, replied, reply_row};
use crate::conversation::flow::tests::{after, layout, stage};

fn drawn_at(conversation: &Conversation, now: Instant) -> Drawn {
    let stage = stage();
    let top = stage.top_row(conversation.reader.focus(now));
    conversation.selection_drawn((&stage, &layout()), top, now, &IdTranslator)
}

fn selected(now: Instant) -> Conversation {
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 2, now), 1, &layout(), now);
    conversation.select_drag(point_at(&conversation, row, 9, now), &layout(), now);
    conversation
}

#[test]
fn washes_the_text_being_selected_in_the_live_colour() {
    let now = after(Instant::now(), 5_000);
    let conversation = selected(now);

    let drawn = drawn_at(&conversation, now);

    assert_eq!(drawn.grounds.len(), 1);
    let wash = drawn.grounds[0];
    assert_eq!(wash.color.ink, Ink::Live);
    assert_eq!(wash.color.alpha, WASH_MAKING);
    let advance = layout().advance();
    assert!((wash.width - (7.0 * advance).round()).abs() <= 1.0);
    assert_eq!(drawn.panes.len(), 0);
}

#[test]
fn shows_a_live_beam_at_each_end_and_brackets_around_the_words_while_dragging() {
    let now = after(Instant::now(), 5_000);
    let conversation = selected(now);

    let drawn = drawn_at(&conversation, now);

    let live = drawn
        .marks
        .iter()
        .filter(|mark| mark.color.ink == Ink::Live)
        .count();
    assert_eq!(live, 2 * 3 + 2);
}

#[test]
fn settles_to_a_faint_white_with_its_size_and_copy_key_once_released() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let settled = drawn_at(&conversation, after(now, 500));

    let wash = settled.grounds[0];
    assert_eq!(wash.color.ink, Ink::Full);
    assert_eq!(wash.color.heat, 0);
    assert_eq!(wash.color.alpha, WASH_MADE);
    assert!(
        settled
            .marks
            .iter()
            .all(|mark| mark.color.ink == MADE_BRACKET_INK)
    );
    let texts: Vec<String> = settled.panes[0]
        .lines
        .iter()
        .map(|placed| {
            placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect()
        })
        .collect();
    assert_eq!(
        texts,
        [
            format!(
                "{}{LABEL_GAP}{} 12",
                Text::SelectionLabel.id(),
                Text::SelectionCharacters.id()
            ),
            format!("{COPY_KEY}{LABEL_GAP}{}", Text::SelectionCopy.id()),
        ]
    );
}

#[test]
fn writes_the_labels_at_sixty_four_percent_and_the_values_in_full_ink() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let settled = drawn_at(&conversation, after(now, 2_000));

    let inks: Vec<Vec<Ink>> = settled.panes[0]
        .lines
        .iter()
        .map(|placed| placed.line.spans.iter().map(|span| span.ink).collect())
        .collect();
    let pane = &settled.panes[0];
    let small = layout().advance_of(TextSize::Small);
    let widest = pane
        .lines
        .iter()
        .map(|placed| {
            usize::from(placed.line.indent)
                + placed.line.spans.iter().map(Span::columns).sum::<usize>()
        })
        .max()
        .unwrap();
    let right = stage().column[0] - stage().points(FROM_THE_COLUMN);
    assert!((pane.left + to_f32(rows(widest)) * small - right).abs() <= 1.0);
    let row = [LABEL_INK, LABEL_INK, VALUE_INK];
    assert_eq!(inks, [row, row]);
    let label = Text::SelectionLabel.id().chars().count();
    assert_eq!(settled.panes[0].lines[0].line.indent, 0);
    assert_eq!(
        usize::from(settled.panes[0].lines[1].line.indent),
        label - COPY_KEY.chars().count()
    );
}

#[test]
fn cools_the_brackets_from_the_live_colour_to_sixty_four_percent_and_warms_them_on_copy() {
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
fn says_copy_again_two_seconds_and_a_half_after_copying() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);
    conversation.mark_copied(now);
    let copy_at = |at: Instant| {
        drawn_at(&conversation, at).panes[0].lines[1].line.spans[2]
            .text
            .clone()
    };

    assert_eq!(copy_at(after(now, 2_300)), Text::SelectionCopied.id());
    assert_eq!(copy_at(after(now, 2_500)), Text::SelectionCopy.id());
    assert!(conversation.selection_moves(after(now, 2_300)));
    assert!(!conversation.selection_moves(after(now, 2_500)));
}

#[test]
fn warms_the_wash_halfway_through_settling() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let halfway = drawn_at(&conversation, after(now, 100)).grounds[0];

    assert!(halfway.color.heat > 0);
    assert!(halfway.color.alpha < WASH_MAKING && halfway.color.alpha > WASH_MADE);
}

#[test]
fn says_copied_warm_then_cooling() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);
    conversation.mark_copied(now);

    let drawn = drawn_at(&conversation, after(now, 100));

    let copy = &drawn.panes[0].lines[1].line.spans[2];
    assert_eq!(copy.text, Text::SelectionCopied.id());
    assert!(copy.heat > 0);
    let cooled = drawn_at(&conversation, after(now, 1_300));
    assert_eq!(cooled.panes[0].lines[1].line.spans[2].heat, 0);
}

#[test]
fn counts_lines_once_the_selection_spans_rows() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.select_press(point_at(&conversation, row, 0, now), 3, &layout(), now);
    conversation
        .selecting
        .as_mut()
        .unwrap()
        .selection
        .anchor
        .row = row - 2;

    let drawn = drawn_at(&conversation, now);

    let size = &drawn.panes[0].lines[0].line.spans[0].text;
    assert_eq!(size, &format!("{} 3", Text::SelectionLines.id()));
    let count = &drawn.panes[0].lines[0].line.spans[2].text;
    assert!(count.starts_with(Text::SelectionCharacters.id()));
}

#[test]
fn shows_a_grey_beam_where_the_pointer_would_start_a_selection() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = replied(Instant::now());
    let row = reply_row(&conversation);
    conversation.point(Some(point_at(&conversation, row, 3, now)));

    let drawn = drawn_at(&conversation, now);

    assert_eq!(drawn.marks.len(), 1);
    assert_eq!(drawn.marks[0].color.ink, BEAM_INK);
    assert!((drawn.marks[0].width - HAIRLINE).abs() < f32::EPSILON);
    conversation.point(Some([1.0, 1.0]));
    assert_eq!(drawn_at(&conversation, now).marks.len(), 0);
    conversation.point(None);
    assert_eq!(drawn_at(&conversation, now).marks.len(), 0);
}

#[test]
fn draws_an_opening_bracket_with_ticks_inward() {
    let [stem, top, bottom] = bracket([10.0, 20.0, 30.0], true, 3.0, Paint::ink(Ink::Live));

    assert!((stem.left - 10.0).abs() < f32::EPSILON && (stem.height - 30.0).abs() < f32::EPSILON);
    assert!((top.left - 10.0).abs() < f32::EPSILON && (top.width - 3.0).abs() < f32::EPSILON);
    assert!((bottom.top - 49.0).abs() < f32::EPSILON);
    let [_, closing, _] = bracket([10.0, 20.0, 30.0], false, 3.0, Paint::ink(Ink::Live));
    assert!((closing.left - 8.0).abs() < f32::EPSILON);
}

#[test]
fn blends_two_alphas() {
    assert_eq!(blend(100, 50, 0.0), 100);
    assert_eq!(blend(100, 50, 0.5), 75);
    assert_eq!(blend(100, 50, 1.0), 50);
    assert_eq!(blend(100, 50, 2.0), 50);
}
