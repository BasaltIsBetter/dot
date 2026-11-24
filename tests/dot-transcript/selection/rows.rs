use std::time::Instant;

use super::*;
use crate::line::TextWidth;

fn narrow(texts: &[&str]) -> Transcript {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    for text in texts {
        transcript.append(text, now);
    }
    transcript.rewrap(TextWidth::new(10), 0.0, now);
    transcript
}

fn texts(transcript: &Transcript) -> Vec<Option<(String, bool)>> {
    (0..transcript.total())
        .map(|row| {
            transcript
                .row_text(row)
                .map(|shown| (shown.text.to_owned(), shown.flows_on))
        })
        .collect()
}

#[test]
fn gives_the_text_of_a_question_without_its_marker() {
    let transcript = narrow(&["hello"]);

    let shown = transcript.row_text(0).unwrap();

    assert_eq!(shown.text, "hello");
    assert_eq!(shown.indent, TEXT_INDENT);
    assert!(!shown.flows_on);
}

#[test]
fn tells_which_rows_go_on_with_the_same_paragraph() {
    let transcript = narrow(&["one two three four\nfive"]);

    let rows: Vec<(String, bool)> = texts(&transcript).into_iter().flatten().collect();

    assert!(rows.len() >= 3, "{rows:?}");
    let last = rows.len() - 1;
    assert!(
        rows[..last - 1].iter().all(|(_, flows_on)| *flows_on),
        "{rows:?}"
    );
    assert!(!rows[last - 1].1, "{rows:?}");
    assert_eq!(rows[last], ("five".to_owned(), false));
}

#[test]
fn has_no_text_for_a_gap_or_past_the_end() {
    let transcript = narrow(&["a", "b"]);

    let rows = texts(&transcript);

    assert!(rows.contains(&None), "{rows:?}");
    assert_eq!(transcript.row_text(transcript.total()), None);
}

#[test]
fn has_no_text_for_a_row_of_code() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("fix it", now);
    transcript.begin_reply(now);
    let tool = crate::transcript::Tool {
        glyph: crate::transcript::Glyph::Edit,
        label: "a.rs".to_owned(),
    };
    transcript.start_tool("t1".into(), tool, now);
    let before = transcript.total();
    let change = crate::code::tests::edited(
        1,
        vec![crate::code::ChangeLine::Added("let a = 1;".to_owned())],
    );

    transcript.change_tool("t1", change, now);

    assert!(transcript.total() > before);
    let code_rows = (before..transcript.total()).filter(|&row| transcript.row_text(row).is_none());
    assert!(code_rows.count() > 0);
}
