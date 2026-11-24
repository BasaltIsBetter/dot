use super::*;
use crate::line::Band;
use crate::transcript::Kind;
use crate::transcript::tool::Tool;
use crate::transcript::view::tests::{LINE, after, row_top, view};

const COLUMNS: usize = 24;

fn tool(glyph: Glyph, label: &str) -> Tool {
    Tool {
        glyph,
        label: label.to_owned(),
    }
}

fn text_of(marked: &Marked) -> String {
    marked
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn replying(start: Instant) -> Transcript {
    let mut transcript = Transcript::default();
    transcript.append("look", start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "Let me look at it.", start);
    transcript
}

#[test]
fn marks_a_running_tool_in_orange_on_the_row_it_was_called_from() {
    let start = Instant::now();
    let mut transcript = replying(start);
    transcript.start_tool("t1".into(), tool(Glyph::Read, "layout.rs"), start);

    let marked = gutter(&transcript, &view(2.0), COLUMNS, after(start, 2_000));

    assert_eq!(marked.len(), 1);
    assert_eq!(marked[0].top, row_top(0.0));
    assert_eq!(marked[0].glyph, Glyph::Read);
    assert_eq!(marked[0].heat, u8::MAX);
    assert_eq!(text_of(&marked[0]), "layout.rs");
    assert_eq!(marked[0].line.indent, 3);
    assert_eq!(marked[0].line.spans[0].ink, Ink::Secondary);
}

#[test]
fn adds_the_result_once_the_tool_ended_and_cools_from_then() {
    let start = Instant::now();
    let mut transcript = replying(start);
    transcript.start_tool("t1".into(), tool(Glyph::Read, "layout.rs"), start);
    transcript.end_tool("t1", "212 l.".to_owned(), None, after(start, 500));

    let ended = gutter(&transcript, &view(2.0), COLUMNS, after(start, 500));
    let cold = gutter(&transcript, &view(2.0), COLUMNS, after(start, 2_000));

    assert_eq!(text_of(&ended[0]), "layout.rs  212 l.");
    assert_eq!(ended[0].heat, u8::MAX);
    assert_eq!(cold[0].heat, 0);
}

#[test]
fn tools_called_from_the_same_row_stack_below_it() {
    let start = Instant::now();
    let mut transcript = replying(start);
    for (index, name) in ["a.rs", "b.rs", "c.rs"].iter().enumerate() {
        transcript.start_tool(format!("t{index}"), tool(Glyph::Read, name), start);
    }

    let marked = gutter(&transcript, &view(2.0), COLUMNS, start);

    let tops: Vec<f32> = marked.iter().map(|marked| marked.top).collect();
    assert_eq!(tops, [row_top(0.0), row_top(1.0), row_top(2.0)]);
}

#[test]
fn keeps_the_result_whole_and_shortens_the_label_to_fit() {
    let start = Instant::now();
    let mut transcript = replying(start);
    transcript.start_tool(
        "t1".into(),
        tool(Glyph::Command, "cargo test -p dot-desktop --all-features"),
        start,
    );
    transcript.end_tool("t1", "✓".to_owned(), None, start);

    let marked = gutter(&transcript, &view(2.0), COLUMNS, start);

    let text = text_of(&marked[0]);
    assert!(text.ends_with("…  ✓"), "{text}");
    assert_eq!(columns(&text), COLUMNS - 3);
}

#[test]
fn leaves_out_the_tools_outside_the_view() {
    let start = Instant::now();
    let mut transcript = replying(start);
    transcript.start_tool("t1".into(), tool(Glyph::Read, "a.rs"), start);

    let above = gutter(&transcript, &view(10.0), COLUMNS, start);
    let below = gutter(&transcript, &view(-10.0), COLUMNS, start);

    assert_eq!(above, Vec::new());
    assert_eq!(below, Vec::new());
}

#[test]
fn an_older_tool_recedes_like_its_row_and_stays_back_while_rereading() {
    let start = Instant::now();
    let mut transcript = replying(start);
    transcript.start_tool("t1".into(), tool(Glyph::Search, "« grid »"), start);
    transcript.end_tool("t1", "×3".to_owned(), None, start);
    transcript.end_reply(None, start);
    transcript.append("next", start);

    let tall = View {
        band: Band {
            top: 0.0,
            bottom: 10.0 * LINE,
        },
        ..view(0.0)
    };
    let marked = gutter(&transcript, &tall, COLUMNS, after(start, 2_000));

    assert_eq!(marked[0].line.alpha, 163);
    let looking = View {
        review: Some(2.0),
        ..tall
    };
    assert_eq!(
        gutter(&transcript, &looking, COLUMNS, after(start, 2_000))[0]
            .line
            .alpha,
        163
    );
}
