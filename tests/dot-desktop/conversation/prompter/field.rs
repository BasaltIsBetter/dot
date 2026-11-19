use winit::dpi::PhysicalSize;

use super::*;

const PLACEHOLDER: &str = "placeholder";
const LINE: f32 = 20.0 * 2.0 * 0.9;
const BAND: Band = Band {
    top: 1_000.0,
    bottom: 1_000.0 + LINE,
};

fn layout() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), 2.0)
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

    assert_eq!(lines[0], (1_000.0 - LINE, 0, "› one".to_owned(), Ink::Full));
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
    assert_eq!(lines[0].0, band(3).top);
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

    prompter.place([column_three, band(2).top + 5.0], band(2), 2, &layout);
    assert_eq!(prompter.draft.caret_cell(), (0, 3));

    prompter.place([layout.x(60), BAND.bottom - 1.0], band(2), 2, &layout);
    assert_eq!(prompter.draft.caret_cell(), (1, 5));

    prompter.place([0.0, 1_200.0], band(2), 2, &layout);
    assert_eq!(prompter.draft.caret_cell(), (1, 0));
}

#[test]
fn sending_hands_the_text_over_and_empties_the_field() {
    let mut prompter = prompter("hello");

    assert_eq!(prompter.take(), Some(("hello".to_owned(), Vec::new())));
    assert_eq!(prompter.rows(), 1);
    assert_eq!(prompter.take(), None);
}

#[test]
fn fills_the_ninety_six_columns_with_the_marker_and_ninety_four_characters() {
    let lines = shown(&prompter(&"a".repeat(102)), 2, true);

    assert_eq!(lines[0].2, format!("› {}", "a".repeat(94)));
    assert_eq!((lines[1].1, lines[1].2.as_str()), (2, "aaaaaaaa"));
}

#[test]
fn carries_spaces_typed_past_a_full_row_onto_the_next_row() {
    let lines = shown(&prompter(&format!("{}   ", "a".repeat(94))), 2, true);

    assert_eq!(lines[1].1, 2);
    assert_eq!(lines[1].2, "   ");
    assert_eq!(lines[2].1, 5);
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
    prompter.place(
        [layout.x(2), band(3).top + 0.625 * LINE],
        band(3),
        3,
        &layout,
    );

    assert!(
        (caret.top - (band(3).top + 2.5 * LINE)).abs() < f32::EPSILON,
        "{caret:?}"
    );
    assert_eq!(prompter.draft.caret_cell(), (9, 0));
}

fn attachment(label: &str) -> Attachment {
    let mut parts = label.split("  ").map(str::to_owned);
    Attachment {
        kind: parts.next().unwrap_or_default(),
        name: parts.next().unwrap_or_default(),
        size: parts.next().unwrap_or_default(),
        image: dot_api::Image::from_bytes(b"\x89PNG\r\n\x1a\n".to_vec()).unwrap(),
    }
}

#[test]
fn shows_each_attached_image_on_a_row_below_the_text() {
    let mut prompter = prompter("look");

    prompter.attach(attachment("image  a.png  2 KB"));
    prompter.attach(attachment("image  b.png  1.2 MB"));

    assert_eq!(prompter.rows(), 3);
    let lines = shown(&prompter, 3, true);
    assert_eq!(lines[1].2, "image  a.png  2 KB");
    assert_eq!(lines[2].2, "image  b.png  1.2 MB");
    assert_eq!((lines[1].1, lines[1].3), (2, Ink::Secondary));
}

#[test]
fn hides_the_placeholder_once_an_image_is_attached() {
    let mut prompter = Prompter::default();

    prompter.attach(attachment("image  a.png  2 KB"));

    let texts: Vec<String> = shown(&prompter, 2, true)
        .into_iter()
        .map(|line| line.2)
        .collect();
    assert!(!texts.contains(&PLACEHOLDER.to_owned()), "{texts:?}");
}

#[test]
fn erasing_at_the_start_of_the_text_removes_the_last_image() {
    let mut prompter = Prompter::default();
    prompter.attach(attachment("first"));
    prompter.attach(attachment("second"));

    prompter.edit(Edit::Erase(Motion::Left));

    assert_eq!(prompter.attachments, [attachment("first")]);
}

#[test]
fn erasing_inside_the_text_keeps_the_images() {
    let mut prompter = prompter("ab");
    prompter.attach(attachment("first"));

    prompter.edit(Edit::Erase(Motion::Left));

    assert_eq!(prompter.draft.text(), "a");
    assert_eq!(prompter.attachments.len(), 1);
}

#[test]
fn clicking_an_image_row_removes_that_image() {
    let layout = layout();
    let mut prompter = prompter("text");
    prompter.attach(attachment("first"));
    prompter.attach(attachment("second"));

    prompter.place([0.0, band(3).top + LINE + 1.0], band(3), 3, &layout);

    assert_eq!(prompter.attachments, [attachment("second")]);
    assert_eq!(prompter.draft.text(), "text");
}

#[test]
fn sends_images_alone_and_clears_a_blank_draft() {
    let mut prompter = prompter("  \n ");
    prompter.attach(attachment("only"));

    let sent = prompter.take();

    assert_eq!(sent, Some((String::new(), vec![attachment("only")])));
    assert_eq!(prompter.draft.text(), "");
    assert_eq!(prompter.rows(), 1);
}

fn narrowest() -> TextWidth {
    TextWidth::of(&Layout::reflowed(PhysicalSize::new(0, 0), 1.0))
}

fn narrowed(text: &str) -> Prompter {
    let mut prompter = prompter(text);
    prompter.rewrap_to(narrowest());
    prompter
}

#[test]
fn cuts_a_placeholder_too_long_for_a_narrow_column() {
    let long = "Write to Claude · return send · shift+return new line";
    let lines = narrowed("")
        .lines(band(1), 1, &layout(), true, long)
        .into_iter()
        .map(|placed| (placed.line.indent, placed.line.spans[0].text.clone()))
        .collect::<Vec<_>>();
    let wide = Prompter::default().lines(band(1), 1, &layout(), true, long);

    assert_eq!(lines[2], (4, "Write to Claude · return …".to_owned()));
    assert_eq!(4 + lines[2].1.chars().count(), 30);
    assert_eq!(wide[2].line.spans[0].text, long);
}

#[test]
fn shortens_the_name_of_an_image_to_fit_a_narrow_column() {
    let mut prompter = narrowed("look");
    let label = "image  a-rather-long-capture-name.png  1.2 MB";

    prompter.attach(attachment(label));

    let row = &shown(&prompter, 2, true)[1].2;
    assert_eq!(row, "image  a-rather-lon…  1.2 MB");
    assert_eq!(row.chars().count(), narrowest().columns());
    assert_eq!(attachment(label).label(TextWidth::default()), label);
}

#[test]
fn carries_spaces_typed_past_a_full_row_onto_the_next_row_of_a_narrow_column() {
    let lines = shown(&narrowed(&format!("{}   ", "a".repeat(28))), 2, true);

    assert_eq!((lines[1].1, lines[1].2.as_str()), (2, "   "));
    assert_eq!(lines[2].1, 5);
}

#[test]
fn counts_the_columns_of_a_single_row_with_its_marker_and_caret() {
    let mut composing = prompter("a");
    composing.compose("^^".to_owned());

    assert_eq!(
        Prompter::default().single_row_columns(PLACEHOLDER),
        Some(4 + 11)
    );
    assert_eq!(
        prompter("hi").single_row_columns(PLACEHOLDER),
        Some(2 + 2 + 1)
    );
    assert_eq!(composing.single_row_columns(PLACEHOLDER), Some(2 + 1 + 2));
}

#[test]
fn counts_only_the_part_of_the_placeholder_that_fits() {
    let long = "x".repeat(200);

    assert_eq!(narrowed("").single_row_columns(&long), Some(30));
}

#[test]
fn has_no_single_row_once_it_wraps_or_holds_an_image() {
    let mut with_image = prompter("look");
    with_image.attach(attachment("image  a.png  2 KB"));

    assert_eq!(prompter("one\ntwo").single_row_columns(PLACEHOLDER), None);
    assert_eq!(
        prompter(&"a".repeat(95)).single_row_columns(PLACEHOLDER),
        None
    );
    assert_eq!(with_image.single_row_columns(PLACEHOLDER), None);
}
