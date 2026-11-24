use std::time::Instant;

use super::*;
use crate::line::TextWidth;

fn spot(row: usize, column: usize) -> Spot {
    Spot { row, column }
}

fn narrow(texts: &[&str]) -> Transcript {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    for text in texts {
        transcript.append(text, now);
    }
    transcript.rewrap(TextWidth::new(12), 0.0, now);
    transcript
}

fn rows_of(transcript: &Transcript) -> Vec<String> {
    (0..transcript.total())
        .map(|row| {
            transcript
                .row_text(row)
                .map_or_else(|| "<gap>".to_owned(), |shown| shown.text.to_owned())
        })
        .collect()
}

#[test]
fn orders_its_ends_whichever_way_it_was_dragged() {
    let backwards = Selection {
        anchor: spot(3, 2),
        head: spot(1, 5),
    };

    assert_eq!(backwards.ends(), (spot(1, 5), spot(3, 2)));
    assert!(!backwards.is_empty());
    assert!(Selection::at(spot(2, 2)).is_empty());
}

#[test]
fn covers_the_columns_of_each_row_it_spans() {
    let selection = Selection {
        anchor: spot(1, 4),
        head: spot(3, 2),
    };

    assert_eq!(selection.columns_on(0), None);
    assert_eq!(selection.columns_on(1), Some((4, usize::MAX)));
    assert_eq!(selection.columns_on(2), Some((0, usize::MAX)));
    assert_eq!(selection.columns_on(3), Some((0, 2)));
    assert_eq!(selection.columns_on(4), None);
    assert_eq!(Selection::at(spot(1, 1)).columns_on(1), None);
}

#[test]
fn finds_the_word_under_a_double_click() {
    let transcript = narrow(&["say hey_you!"]);
    assert_eq!(rows_of(&transcript)[0], "say hey_you!");

    let word = word_at(&transcript, spot(0, 6));

    assert_eq!(word.text(&transcript), "hey_you");
    assert_eq!(
        word_at(&transcript, spot(0, 4)).text(&transcript),
        "hey_you"
    );
    assert_eq!(word_at(&transcript, spot(0, 0)).text(&transcript), "say");
}

#[test]
fn takes_a_single_sign_when_the_double_click_is_not_on_a_word() {
    let transcript = narrow(&["a, b"]);

    let sign = word_at(&transcript, spot(0, 1));

    assert_eq!(sign.text(&transcript), ",");
    assert!(word_at(&transcript, spot(0, 40)).is_empty());
}

#[test]
fn a_double_click_on_a_gap_selects_nothing() {
    let transcript = narrow(&["a", "b"]);
    let gap = rows_of(&transcript)
        .iter()
        .position(|row| row == "<gap>")
        .unwrap();

    assert!(word_at(&transcript, spot(gap, 0)).is_empty());
    assert!(Selection::at(spot(gap, 0)).to_words(&transcript).is_empty());
}

#[test]
fn grows_to_whole_words_at_both_ends_and_keeps_its_direction() {
    let transcript = narrow(&["alpha beta gamma"]);
    let rows = rows_of(&transcript);
    assert_eq!(rows[0], "alpha beta ");

    let forward = Selection {
        anchor: spot(0, 2),
        head: spot(0, 8),
    }
    .to_words(&transcript);
    let backward = Selection {
        anchor: spot(0, 8),
        head: spot(0, 2),
    }
    .to_words(&transcript);

    assert_eq!(forward.anchor, spot(0, 0));
    assert_eq!(forward.head, spot(0, 10));
    assert_eq!(backward.anchor, spot(0, 10));
    assert_eq!(backward.head, spot(0, 0));
}

#[test]
fn leaves_an_end_already_between_two_words_where_it_is() {
    let transcript = narrow(&["alpha beta"]);

    let selection = Selection {
        anchor: spot(0, 5),
        head: spot(0, 6),
    }
    .to_words(&transcript);

    assert_eq!(selection.ends(), (spot(0, 5), spot(0, 6)));
}

#[test]
fn copies_a_paragraph_on_one_line_and_paragraphs_on_their_own_lines() {
    let transcript = narrow(&["one two three four\nfive"]);
    let last = transcript.total() - 1;
    let all = Selection {
        anchor: spot(0, 0),
        head: spot(last, 4),
    };

    assert_eq!(all.text(&transcript), "one two three four\nfive");
}

#[test]
fn copies_a_new_line_for_each_gap_between_entries() {
    let transcript = narrow(&["first", "second"]);
    let last = transcript.total() - 1;
    let all = Selection {
        anchor: spot(0, 0),
        head: spot(last, 6),
    };

    assert_eq!(all.text(&transcript), "first\n\nsecond");
}

#[test]
fn copies_only_the_selected_part_of_its_first_and_last_rows() {
    let transcript = narrow(&["alpha beta gamma delta"]);
    let selection = Selection {
        anchor: spot(1, 3),
        head: spot(0, 6),
    };

    assert_eq!(selection.text(&transcript), "beta gam");
}

#[test]
fn counts_its_characters_and_its_rows() {
    let transcript = narrow(&["alpha beta gamma delta"]);
    let selection = Selection {
        anchor: spot(0, 6),
        head: spot(1, 3),
    };

    assert_eq!(selection.size(&transcript), (8, 2));
    assert_eq!(
        Selection {
            anchor: spot(0, 0),
            head: spot(0, 5),
        }
        .size(&transcript),
        (5, 1)
    );
}

#[test]
fn selects_every_row_of_a_paragraph_on_a_triple_click() {
    let transcript = narrow(&["one two three four\nfive"]);
    let last = transcript.total() - 1;

    let paragraph = paragraph_at(&transcript, 1);

    assert_eq!(paragraph.text(&transcript), "one two three four");
    assert_eq!(paragraph.anchor, spot(0, 0));
    assert_eq!(paragraph_at(&transcript, last).text(&transcript), "five");
}

#[test]
fn counts_the_rows_of_a_selection_that_starts_further_down() {
    let transcript = narrow(&["one two three four five six seven"]);
    assert_eq!(
        rows_of(&transcript),
        ["one two ", "three four ", "five six ", "seven"]
    );
    let selection = Selection {
        anchor: spot(1, 0),
        head: spot(2, 4),
    };

    assert_eq!(selection.size(&transcript), (15, 2));
}

#[test]
fn selects_the_same_paragraph_from_its_first_row() {
    let transcript = narrow(&["one two three four\nfive"]);

    let from_the_first_row = paragraph_at(&transcript, 0);

    assert_eq!(from_the_first_row, paragraph_at(&transcript, 1));
    assert_eq!(from_the_first_row.head, spot(1, 10));
}

#[test]
fn keeps_an_end_at_either_edge_of_a_row_where_it_is() {
    let transcript = narrow(&["alpha beta gamma delta"]);
    assert_eq!(rows_of(&transcript), ["alpha beta ", "gamma delta"]);

    let to_the_next_row = Selection {
        anchor: spot(0, 7),
        head: spot(1, 0),
    }
    .to_words(&transcript);
    let to_the_end = Selection {
        anchor: spot(1, 7),
        head: spot(1, 11),
    }
    .to_words(&transcript);

    assert_eq!(to_the_next_row.ends(), (spot(0, 6), spot(1, 0)));
    assert_eq!(to_the_end.ends(), (spot(1, 6), spot(1, 11)));
}
