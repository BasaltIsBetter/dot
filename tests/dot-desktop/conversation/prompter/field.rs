use std::time::{Duration, Instant};

use super::*;
use crate::app::tests::IdTranslator;
use crate::conversation::prompter::attachment::tests::attachment;
use crate::conversation::suggestion::Suggestions;

const ROW: f32 = 35.2;
const TOP: f32 = 1_000.0;
const HINT: &str = "suggestion-explain-project";

fn top_of(row: usize) -> f32 {
    TOP + ROW * f32::from(u16::try_from(row).unwrap())
}

fn prompter(text: &str) -> Prompter {
    let mut prompter = Prompter::default();
    prompter.edit(Edit::Insert(text.to_owned()), Instant::now());
    prompter
}

fn hint() -> Hint {
    Suggestions::default().hint(Instant::now(), &IdTranslator)
}

fn later() -> Instant {
    Instant::now() + Duration::from_secs(1)
}

fn shown(prompter: &Prompter, caret: Caret, hint: Option<&Hint>) -> Vec<(f32, u16, String, Ink)> {
    prompter
        .lines((top_of, ROW), (caret, Mark::Draft), hint, later())
        .into_iter()
        .map(|placed| {
            let text = placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            (
                placed.top,
                placed.line.indent,
                text,
                placed.line.spans[0].ink,
            )
        })
        .collect()
}

fn texts(prompter: &Prompter, caret: Caret) -> Vec<String> {
    shown(prompter, caret, None)
        .into_iter()
        .map(|(_, _, text, _)| text)
        .collect()
}

#[test]
fn an_empty_prompter_shows_its_marker_then_the_suggestion_with_the_caret_after_it() {
    let hint = hint();

    let lines = shown(&Prompter::default(), Caret::Lit, Some(&hint));

    assert_eq!(
        lines,
        vec![
            (TOP, 0, "›".to_owned(), Ink::Receding),
            (TOP, TEXT_INDENT, HINT.to_owned(), Ink::Receding),
            (TOP, TEXT_INDENT + 26, "_".to_owned(), Ink::Full),
        ]
    );
}

#[test]
fn an_empty_prompter_without_a_suggestion_shows_only_its_marker() {
    let lines = shown(&Prompter::default(), Caret::Lit, None);

    assert_eq!(lines, vec![(TOP, 0, "›".to_owned(), Ink::Receding)]);
}

#[test]
fn shows_what_is_typed_with_a_receding_marker_and_the_caret_after_it() {
    let lines = shown(&prompter("hello"), Caret::Lit, None);

    assert_eq!(
        lines,
        vec![
            (TOP, 0, "› hello".to_owned(), Ink::Receding),
            (TOP, TEXT_INDENT + 5, "_".to_owned(), Ink::Full),
        ]
    );
}

#[test]
fn what_is_typed_arrives_orange() {
    let mut prompter = Prompter::default();
    let typed = Instant::now();
    prompter.edit(Edit::Insert("hi".to_owned()), typed);

    let lines = prompter.lines((top_of, ROW), (Caret::Lit, Mark::Draft), None, typed);

    assert_eq!(lines[0].line.spans[1].heat, u8::MAX);
    assert!(prompter.is_cooling(typed));
    assert!(!prompter.is_cooling(typed + Duration::from_millis(450)));
}

#[test]
fn indents_the_following_rows_under_the_text() {
    let lines = shown(&prompter("first\nsecond"), Caret::Lit, None);

    assert_eq!(
        lines[1],
        (top_of(1), TEXT_INDENT, "second".to_owned(), Ink::Full)
    );
    assert_eq!(
        lines[2],
        (top_of(1), TEXT_INDENT + 6, "_".to_owned(), Ink::Full)
    );
}

#[test]
fn the_caret_recedes_in_the_background_and_hides_while_it_blinks_off() {
    let receded = shown(&prompter("hi"), Caret::Receded, None);
    let dark = texts(&prompter("hi"), Caret::Dark);

    assert_eq!(receded[1].3, Ink::Receding);
    assert_eq!(dark, vec!["› hi"]);
}

#[test]
fn shows_a_composed_accent_in_orange_in_place_of_the_caret_even_while_it_blinks_off() {
    let mut prompter = prompter("caf");
    prompter.compose("´".to_owned());

    let lines = shown(&prompter, Caret::Dark, None);

    assert_eq!(lines[1], (TOP, TEXT_INDENT + 3, "´".to_owned(), Ink::Live));
    assert!(!prompter.is_empty());
}

#[test]
fn clicking_a_cell_places_the_caret_there() {
    let mut prompter = prompter("hello\nworld");

    prompter.place(1, TEXT_INDENT as usize + 2);

    assert_eq!(prompter.caret_cell(), (1, 2));
}

#[test]
fn sending_hands_the_text_over_and_empties_the_field() {
    let mut prompter = prompter("  hello  ");

    let taken = prompter.take();

    assert_eq!(taken, Some(("  hello".to_owned(), Vec::new())));
    assert!(prompter.is_empty());
    assert_eq!(prompter.take(), None);
}

#[test]
fn shows_each_attached_image_on_a_row_below_the_text() {
    let mut prompter = prompter("look");
    prompter.attach(attachment("a.png")).unwrap();
    prompter.attach(attachment("b.png")).unwrap();

    let lines = shown(&prompter, Caret::Dark, None);

    assert_eq!(prompter.rows(), 3);
    assert_eq!(
        lines[1],
        (
            top_of(1),
            TEXT_INDENT,
            "image  a.png  2 KB".to_owned(),
            Ink::Secondary
        )
    );
    assert_eq!(lines[2].0, top_of(2));
}

#[test]
fn shows_no_suggestion_once_an_image_is_attached() {
    let mut prompter = Prompter::default();
    prompter.attach(attachment("a.png")).unwrap();
    let hint = hint();

    let lines = shown(&prompter, Caret::Dark, Some(&hint));

    assert!(!prompter.is_empty());
    assert!(lines.iter().all(|(_, _, text, _)| text != HINT));
}

#[test]
fn erasing_at_the_start_of_the_text_removes_the_last_image() {
    let mut prompter = Prompter::default();
    prompter.attach(attachment("a.png")).unwrap();
    prompter.attach(attachment("b.png")).unwrap();

    prompter.edit(Edit::Erase(Motion::Left), Instant::now());

    assert_eq!(prompter.rows(), 2);
}

#[test]
fn erasing_inside_the_text_keeps_the_images() {
    let mut prompter = prompter("ab");
    prompter.attach(attachment("a.png")).unwrap();

    prompter.edit(Edit::Erase(Motion::Left), Instant::now());

    assert_eq!(
        texts(&prompter, Caret::Dark),
        vec!["› a", "image  a.png  2 KB"]
    );
}

#[test]
fn clicking_an_image_row_removes_that_image_and_a_row_past_them_nothing() {
    let mut prompter = prompter("look");
    prompter.attach(attachment("a.png")).unwrap();
    prompter.attach(attachment("b.png")).unwrap();

    prompter.place(1, 4);
    prompter.place(9, 4);

    assert_eq!(texts(&prompter, Caret::Dark)[1], "image  b.png  2 KB");
    assert_eq!(prompter.rows(), 2);
}

#[test]
fn sends_images_alone_and_clears_a_blank_draft() {
    let mut prompter = prompter("   ");
    prompter.attach(attachment("a.png")).unwrap();

    let taken = prompter.take();

    assert_eq!(
        taken.map(|(text, images)| (text, images.len())),
        Some((String::new(), 1))
    );
    assert!(prompter.is_empty());
}

#[test]
fn refuses_an_image_past_twenty() {
    let mut prompter = Prompter::default();
    for _ in 0..20 {
        prompter.attach(attachment("a.png")).unwrap();
    }

    assert!(prompter.attach(attachment("a.png")).is_err());
}

#[test]
fn cuts_a_suggestion_too_long_for_a_narrow_column() {
    let mut prompter = Prompter::default();
    prompter.rewrap_to(TextWidth::new(10));
    let hint = hint();

    let lines = shown(&prompter, Caret::Lit, Some(&hint));

    assert_eq!(lines[1].2, "suggesti…");
    assert_eq!(lines[2].1, TEXT_INDENT + 9);
}

#[test]
fn shows_a_typed_tab_as_four_spaces_and_keeps_it_in_the_text() {
    let mut prompter = prompter("a\tb");

    let lines = texts(&prompter, Caret::Dark);

    assert_eq!(lines, vec!["› a    b"]);
    assert_eq!(
        prompter.take().map(|(text, _)| text),
        Some("a\tb".to_owned())
    );
}

#[test]
fn marks_a_note_in_orange_with_its_caret_and_its_hint_in_secondary_ink() {
    let note = prompter("shorter");
    let hint = Hint::still("a note".to_owned());

    let written = note.lines((top_of, ROW), (Caret::Lit, Mark::Note), None, later());
    let empty = Prompter::default().lines(
        (top_of, ROW),
        (Caret::Dark, Mark::Note),
        Some(&hint),
        later(),
    );

    assert_eq!(written[0].line.spans[0].text, "‖ ");
    assert_eq!(written[0].line.spans[0].ink, Ink::Live);
    assert_eq!(written[1].line.spans[0].ink, Ink::Live);
    assert_eq!(empty[0].line.spans[0].text, "‖");
    assert_eq!(empty[1].line.spans[0].ink, Ink::Secondary);
    assert_eq!(empty.len(), 2);
}

#[test]
fn a_click_below_the_last_image_takes_none_away() {
    let mut prompter = prompter("look");
    prompter.attach(attachment("a.png")).unwrap();

    prompter.place(2, 0);

    assert_eq!(prompter.rows(), 2);
    let taken = prompter.take().unwrap();
    assert_eq!(taken.1.len(), 1);
}
