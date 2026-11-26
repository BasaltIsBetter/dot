use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::flow::select::draw::Drawn;
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
fn writes_its_size_and_copy_key_once_released() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let settled = drawn_at(&conversation, after(now, 500));

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
fn sets_its_two_lines_a_row_apart_from_the_first_selected_row() {
    let now = after(Instant::now(), 5_000);
    let mut conversation = selected(now);
    conversation.select_release(now);

    let pane = &drawn_at(&conversation, after(now, 500)).panes[0];

    let stage = stage();
    let top = stage.row_top(
        reply_row(&conversation),
        stage.top_row(conversation.reader.focus(now)),
    );
    let expected = layout().text_top(TextSize::Small, top, stage.row);
    assert!((pane.lines[0].top - expected).abs() < 0.5);
    assert!((pane.lines[1].top - pane.lines[0].top - stage.row).abs() < 0.5);
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
