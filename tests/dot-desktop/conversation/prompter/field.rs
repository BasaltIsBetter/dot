use winit::dpi::PhysicalSize;

use super::*;

const PLACEHOLDER: &str = "placeholder";
const LINE: f32 = 40.0;
const BAND: Band = Band {
    top: 1_000.0,
    bottom: 1_040.0,
};

fn layout() -> Layout {
    Layout::new(PhysicalSize::new(2880, 1800), 2.0)
}

fn prompter(text: &str) -> Prompter {
    let mut prompter = Prompter::default();
    prompter.edit(Edit::Insert(text.to_owned()));
    prompter
}

fn band(shown: usize) -> Band {
    Band {
        top: BAND.bottom - LINE * f32::from(u8::try_from(shown).unwrap()),
        bottom: BAND.bottom,
    }
}

fn shown(prompter: &Prompter, rows: usize, focused: bool) -> Vec<(f32, u16, String, Ink)> {
    prompter
        .lines(band(rows), rows, &layout(), focused, PLACEHOLDER)
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

#[test]
fn an_empty_prompter_shows_its_marker_caret_and_placeholder() {
    let lines = shown(&Prompter::default(), 1, true);

    assert_eq!(
        lines,
        vec![
            (1_000.0, 0, "› ".to_owned(), Ink::Full),
            (1_000.0, 2, "_".to_owned(), Ink::Full),
            (1_000.0, 4, PLACEHOLDER.to_owned(), Ink::Receding),
        ]
    );
}

#[test]
fn shows_what_is_typed_with_the_caret_after_it() {
    let lines = shown(&prompter("hello"), 1, true);

    assert_eq!(
        lines,
        vec![
            (1_000.0, 0, "› hello".to_owned(), Ink::Full),
            (1_000.0, 7, "_".to_owned(), Ink::Full),
        ]
    );
}

#[test]
fn indents_the_following_rows_under_the_text() {
    let lines = shown(&prompter("one\ntwo"), 2, true);

    assert_eq!(lines[0], (960.0, 0, "› one".to_owned(), Ink::Full));
    assert_eq!(lines[1], (1_000.0, 2, "two".to_owned(), Ink::Full));
    assert_eq!(lines[2], (1_000.0, 5, "_".to_owned(), Ink::Full));
}

#[test]
fn the_caret_recedes_while_the_window_is_in_the_background() {
    let lines = shown(&prompter("hi"), 1, false);

    assert_eq!(lines[1], (1_000.0, 4, "_".to_owned(), Ink::Receding));
}

#[test]
fn shows_a_composed_accent_in_orange_in_place_of_the_caret() {
    let mut prompter = Prompter::default();

    prompter.compose("^".to_owned());

    let lines = shown(&prompter, 1, true);
    assert_eq!(
        lines,
        vec![
            (1_000.0, 0, "› ".to_owned(), Ink::Full),
            (1_000.0, 2, "^".to_owned(), Ink::Live),
        ]
    );
}

#[test]
fn shows_only_the_rows_that_fit_and_follows_the_caret() {
    let text = (1..=12)
        .map(|row| format!("row {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut prompter = prompter(&text);

    prompter.reveal_caret(3);

    let lines = shown(&prompter, 3, true);
    let rows: Vec<&str> = lines.iter().map(|(_, _, text, _)| text.as_str()).collect();
    assert_eq!(rows, vec!["row 10", "row 11", "row 12", "_"]);
    assert_eq!(lines[0].0, 920.0);
}

#[test]
fn scrolls_up_to_earlier_rows_and_hides_a_caret_out_of_view() {
    let text = (1..=12)
        .map(|row| format!("row {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut prompter = prompter(&text);

    prompter.scroll_by(100.0, 3);

    let lines = shown(&prompter, 3, true);
    let rows: Vec<&str> = lines.iter().map(|(_, _, text, _)| text.as_str()).collect();
    assert_eq!(rows, vec!["› row 1", "row 2", "row 3"]);
}

#[test]
fn moving_the_caret_out_of_view_brings_it_back() {
    let text = (1..=12)
        .map(|row| format!("row {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut prompter = prompter(&text);
    prompter.scroll_by(100.0, 3);

    prompter.edit(Edit::Move(
        crate::conversation::prompter::motion::Motion::End,
    ));
    prompter.reveal_caret(3);

    let lines = shown(&prompter, 3, true);
    assert_eq!(lines[2].2, "row 12");
}

#[test]
fn clicking_a_cell_places_the_caret_there() {
    let layout = layout();
    let mut prompter = prompter("hello\nworld");
    let column_three = layout.x(2 + 3) + 1.0;

    prompter.place([column_three, 965.0], band(2), 2, &layout);
    assert_eq!(prompter.draft.caret_cell(), (0, 3));

    prompter.place([layout.x(60), 1_039.0], band(2), 2, &layout);
    assert_eq!(prompter.draft.caret_cell(), (1, 5));

    prompter.place([0.0, 1_200.0], band(2), 2, &layout);
    assert_eq!(prompter.draft.caret_cell(), (1, 0));
}

#[test]
fn sending_hands_the_text_over_and_empties_the_field() {
    let mut prompter = prompter("hello");

    assert_eq!(prompter.take(), Some("hello".to_owned()));
    assert_eq!(prompter.rows(), 1);
    assert_eq!(prompter.take(), None);
}

#[test]
fn fills_the_sixty_four_columns_with_the_marker_and_sixty_two_characters() {
    let lines = shown(&prompter(&"a".repeat(70)), 2, true);

    assert_eq!(lines[0].2, format!("› {}", "a".repeat(62)));
    assert_eq!((lines[1].1, lines[1].2.as_str()), (2, "aaaaaaaa"));
}

#[test]
fn keeps_the_caret_inside_the_column_after_hanging_spaces() {
    let lines = shown(&prompter(&format!("{}   ", "a".repeat(62))), 1, true);

    assert_eq!(lines[1], (1_000.0, 64, "_".to_owned(), Ink::Full));
}

#[test]
fn clicks_and_reports_the_caret_in_a_prompter_scrolled_by_half_a_row() {
    let layout = layout();
    let text = (1..=12)
        .map(|row| format!("row {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut prompter = prompter(&text);
    prompter.scroll_by(0.5, 3);

    let caret = prompter.caret_area(band(3), 3, &layout);
    prompter.place([layout.x(2), 945.0], band(3), 3, &layout);

    assert!((caret.top - 1_020.0).abs() < f32::EPSILON, "{caret:?}");
    assert_eq!(prompter.draft.caret_cell(), (9, 0));
}
