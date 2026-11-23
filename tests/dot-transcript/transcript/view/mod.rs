use std::time::Duration;

use super::*;
use crate::code::tests::Words;
use crate::ink::{Ink, OPAQUE};
use crate::transcript::entry::Kind;

pub(super) const LINE: f32 = 22.0 * 2.0 * 0.8;
pub(super) const BAND: Band = Band {
    top: 100.0,
    bottom: 100.0 + 4.0 * LINE,
};
pub(super) const TRAVEL: f32 = 0.7;

pub(super) fn row_top(rows_down: f32) -> f32 {
    (BAND.top + rows_down * LINE).round()
}

pub(super) fn view(top: f64) -> View {
    View {
        band: BAND,
        line_height: LINE,
        top,
        review: None,
        caret_lit: true,
        hovered: None,
    }
}

pub(super) fn on_the_reply(transcript: &Transcript) -> View {
    view(crate::reading::rows(transcript.anchor().unwrap()))
}

pub(super) fn texts(placed: &[Placed]) -> Vec<(f32, u16, String)> {
    placed
        .iter()
        .map(|placed| {
            let text = placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            (placed.top, placed.line.indent, text)
        })
        .collect()
}

pub(super) fn alphas(placed: &[Placed]) -> Vec<u8> {
    placed.iter().map(|placed| placed.line.alpha).collect()
}

#[test]
fn shows_each_entry_with_its_marker_and_a_blank_row_between() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("first", sent);
    transcript.append("second\nline", sent);

    let placed = lines(
        &transcript,
        &view(0.0),
        sent + Duration::from_secs(2),
        &Words,
    );

    assert_eq!(
        texts(&placed),
        vec![
            (row_top(0.0), 0, "› first".to_owned()),
            (row_top(2.0), 0, "› second".to_owned()),
            (row_top(3.0), 2, "line".to_owned()),
        ]
    );
}

#[test]
fn settled_entries_have_a_receding_marker_and_full_ink_text() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello", sent);

    let placed = lines(
        &transcript,
        &view(0.0),
        sent + Duration::from_secs(2),
        &Words,
    );

    let spans = &placed[0].line.spans;
    assert_eq!((spans[0].ink, spans[0].heat), (Ink::Receding, 0));
    assert_eq!((spans[1].ink, spans[1].heat), (Ink::Full, 0));
}

#[test]
fn a_new_entry_arrives_orange_and_cools() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello\nworld", sent);

    let arriving = lines(&transcript, &view(0.0), sent, &Words);
    let cooling = lines(
        &transcript,
        &view(0.0),
        sent + Duration::from_millis(600),
        &Words,
    );

    let heats = |placed: &[Placed]| -> Vec<u8> {
        placed
            .iter()
            .flat_map(|placed| &placed.line.spans)
            .map(|span| span.heat)
            .collect()
    };
    assert!(heats(&arriving).iter().all(|&heat| heat == u8::MAX));
    assert!(heats(&cooling).iter().all(|&heat| heat == 80));
}

#[test]
fn draws_only_the_rows_inside_the_band() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..100 {
        transcript.append(&format!("entry {number}"), sent);
    }

    let placed = lines(&transcript, &view(195.0), sent, &Words);

    assert_eq!(
        texts(&placed)
            .into_iter()
            .map(|(top, _, text)| (top, text))
            .collect::<Vec<_>>(),
        vec![
            (row_top(1.0), "› entry 98".to_owned()),
            (row_top(3.0), "› entry 99".to_owned()),
        ]
    );
}

#[test]
fn places_rows_between_pixels_when_the_top_is_part_of_a_row() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..10 {
        transcript.append(&format!("entry {number}"), sent);
    }

    let placed = lines(&transcript, &view(14.5), sent, &Words);

    assert_eq!(placed[0].top, row_top(-0.5));
    assert_eq!(texts(&placed)[0].2, "› entry 7");
}

#[test]
fn rows_above_the_first_start_lower_in_the_band() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("only", sent);

    let placed = lines(&transcript, &view(-2.0), sent, &Words);

    assert_eq!(texts(&placed), vec![(row_top(2.0), 0, "› only".to_owned())]);
}

pub(super) fn inks(placed: &Placed) -> Vec<Ink> {
    placed.line.spans.iter().map(|span| span.ink).collect()
}

#[test]
fn draws_an_orange_caret_after_the_reply_while_claude_writes() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.begin_reply(now);
    let waiting = lines(&transcript, &view(0.0), now, &Words);

    transcript.hear(Kind::Answer, "Hi", now);
    let streaming = lines(&transcript, &view(0.0), now, &Words);
    transcript.end_reply(None, now);
    let finished = lines(&transcript, &view(0.0), now, &Words);

    assert_eq!(waiting, Vec::new());
    assert_eq!(texts(&streaming).last().unwrap().2, "Hi_");
    assert_eq!(inks(streaming.last().unwrap()).last(), Some(&Ink::Live));
    assert_eq!(texts(&finished).last().unwrap().2, "Hi");
}

#[test]
fn writes_thinking_in_secondary_ink_and_notices_receding() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    let tall = View {
        band: Band {
            top: 100.0,
            bottom: 500.0,
        },
        ..view(0.0)
    };
    transcript.append("hello", now);
    transcript.begin_reply(now);
    transcript.hear(Kind::Thinking, "Hm.", now);
    transcript.hear(Kind::Answer, "Hi", now);
    transcript.end_reply(Some("reply-stopped"), now);

    let placed = lines(&transcript, &tall, now + Duration::from_secs(2), &Words);

    assert_eq!(placed.len(), 4);
    assert_eq!(inks(&placed[1]), [Ink::Secondary]);
    assert_eq!(inks(&placed[2]), [Ink::Full]);
    assert_eq!(inks(&placed[3]), [Ink::Receding]);
    assert!(placed[1..].iter().all(|placed| placed.line.indent == 2));
}

#[test]
fn heats_each_piece_by_its_own_arrival() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.begin_reply(sent);
    transcript.hear(Kind::Answer, "old ", sent);
    transcript.hear(Kind::Answer, "new", sent + Duration::from_millis(600));

    let placed = lines(
        &transcript,
        &view(0.0),
        sent + Duration::from_millis(600),
        &Words,
    );

    let heats: Vec<u8> = placed[0].line.spans.iter().map(|span| span.heat).collect();
    assert_eq!(heats, [80, u8::MAX, 0]);
}

pub(super) fn after(start: Instant, milliseconds: u64) -> Instant {
    start + Duration::from_millis(milliseconds)
}

pub(super) fn asked(question: &str, start: Instant) -> Transcript {
    let mut transcript = Transcript::default();
    transcript.append(question, start);
    transcript.begin_reply(start);
    transcript
}

#[test]
fn the_reply_settles_in_place_and_the_reached_words_are_gone() {
    let start = Instant::now();
    let mut transcript = asked("hello you", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hey", written);

    let placed = lines(&transcript, &view(0.0), after(written, 300), &Words);
    assert_eq!(
        texts(&placed),
        vec![(100.0, 2, "Hey".to_owned()), (100.0, 8, "you".to_owned())]
    );
}

#[test]
fn the_reply_rises_in_fading_while_it_replaces_the_question() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hello there", written);
    transcript.hear(Kind::Answer, " friend", after(written, 1));

    let placed = lines(
        &transcript,
        &on_the_reply(&transcript),
        after(written, 100),
        &Words,
    );
    let replies = &texts(&placed)[..2];
    assert!(replies[0].0 > 100.0 && replies[0].0 < 100.0 + TRAVEL * LINE);
    assert_eq!((replies[0].1, replies[0].2.as_str()), (2, "Hello there"));
    assert!(placed[0].line.alpha > 0 && placed[0].line.alpha < OPAQUE);
    assert_eq!(replies[1], (100.0, 13, " friend".to_owned()));
    assert_eq!(placed[1].line.alpha, OPAQUE);
}

#[test]
fn ends_the_reply_with_the_caret_once_the_question_has_given_way() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hello there", written);

    let rolling = lines(
        &transcript,
        &on_the_reply(&transcript),
        after(written, 299),
        &Words,
    );
    let settled = lines(
        &transcript,
        &on_the_reply(&transcript),
        after(written, 300),
        &Words,
    );
    assert_eq!(texts(&rolling)[0].2, "Hello there");
    assert_eq!(texts(&settled)[0], (100.0, 2, "Hello there_".to_owned()));
}

#[test]
fn leaves_the_caret_out_while_it_blinks_off() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);
    let written = after(start, 600);
    transcript.hear(Kind::Answer, "Hello there", written);

    let dark = View {
        caret_lit: false,
        hovered: None,
        ..on_the_reply(&transcript)
    };
    let placed = lines(&transcript, &dark, after(written, 300), &Words);

    assert_eq!(texts(&placed)[0], (100.0, 2, "Hello there".to_owned()));
}

fn two_exchanges(start: Instant) -> Transcript {
    let mut transcript = Transcript::default();
    transcript.append("old question", start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "old answer is long enough", start);
    transcript.end_reply(None, start);
    transcript.append("new question", start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "new answer is long enough", start);
    transcript.end_reply(None, start);
    transcript
}

#[test]
fn older_exchanges_recede_to_sixty_four_percent() {
    let start = Instant::now();
    let transcript = two_exchanges(start);
    let tall = View {
        band: Band {
            top: 0.0,
            bottom: 20.0 * LINE,
        },
        ..view(0.0)
    };

    let placed = lines(&transcript, &tall, after(start, 3_000), &Words);

    assert_eq!(
        texts(&placed)
            .iter()
            .map(|(_, _, text)| text.clone())
            .collect::<Vec<_>>(),
        [
            "› old question",
            "old answer is long enough",
            "› new question",
            "new answer is long enough"
        ]
    );
    assert_eq!(alphas(&placed), [163, 163, OPAQUE, OPAQUE]);
}

#[test]
fn rows_around_the_reading_line_read_in_full_ink_while_looking_back() {
    let start = Instant::now();
    let transcript = two_exchanges(start);
    let looking_back = View {
        band: Band {
            top: 0.0,
            bottom: 20.0 * LINE,
        },
        review: Some(0.0),
        ..view(0.0)
    };

    let placed = lines(&transcript, &looking_back, after(start, 3_000), &Words);

    assert_eq!(alphas(&placed), [OPAQUE, OPAQUE, OPAQUE, OPAQUE]);
    let far = View {
        review: Some(4.0),
        ..looking_back
    };
    assert_eq!(
        alphas(&lines(&transcript, &far, after(start, 3_000), &Words)),
        [163, OPAQUE, OPAQUE, OPAQUE]
    );
}

#[test]
fn the_question_given_back_fades_in_above_the_reply() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);
    transcript.hear(Kind::Answer, "Hello there", start);

    let waiting = lines(&transcript, &view(0.0), after(start, 200), &Words);
    let fading = lines(&transcript, &view(0.0), after(start, 450), &Words);
    let back = lines(&transcript, &view(0.0), after(start, 700), &Words);

    let question = |placed: &[Placed]| {
        placed
            .iter()
            .find(|placed| texts(std::slice::from_ref(placed))[0].2 == "› hi")
            .map(|placed| placed.line.alpha)
    };
    assert_eq!(question(&waiting), Some(0));
    let halfway = question(&fading).unwrap();
    assert!(halfway > 0 && halfway < OPAQUE, "{halfway}");
    assert_eq!(question(&back), Some(OPAQUE));
}

#[test]
fn the_question_given_back_never_rises_with_the_reply() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);
    transcript.hear(Kind::Answer, "Hello there, friend.", after(start, 10));
    let view = view(0.0);
    let question_top = |milliseconds| {
        lines(&transcript, &view, after(start, milliseconds), &Words)
            .into_iter()
            .find(|placed| placed.line.spans[0].text == MARKER)
            .map(|placed| placed.top)
    };

    assert_eq!(question_top(100), question_top(2_000));
}

#[test]
fn marks_a_note_like_a_prompt_with_its_own_mark() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello", now);
    transcript.note("shorter", now);

    let placed = lines(&transcript, &view(0.0), after(now, 5_000), &Words);

    let shown: Vec<String> = texts(&placed)
        .into_iter()
        .map(|(_, _, text)| text)
        .collect();
    assert_eq!(shown, ["› hello", "‖ shorter"]);
    assert_eq!(placed[1].line.spans[0].ink, Ink::Receding);
    assert_eq!(marker_of(Kind::Answer), None);
}

fn edited_reply(start: Instant) -> Transcript {
    edited_reply_to(start, "new")
}

fn edited_reply_to(start: Instant, new: &str) -> Transcript {
    let mut transcript = Transcript::default();
    transcript.append("fix", start);
    transcript.begin_reply(start);
    transcript.hear(Kind::Answer, "Done.", start);
    let tool = crate::transcript::Tool {
        glyph: crate::transcript::Glyph::Edit,
        label: "a.rs".to_owned(),
    };
    transcript.start_tool("t1".into(), tool, start);
    let change = crate::code::Change {
        path: "a.rs".to_owned(),
        created: false,
        hunks: vec![crate::code::ChangeHunk {
            new_start: 1,
            lines: vec![
                crate::code::ChangeLine::Removed("old".to_owned()),
                crate::code::ChangeLine::Added(new.to_owned()),
            ],
        }],
        neighbours: None,
    };
    transcript.change_tool("t1", change, start);
    transcript
}

fn tall(hovered: Option<usize>) -> View {
    View {
        band: Band {
            top: 0.0,
            bottom: 40.0 * LINE,
        },
        hovered,
        ..view(0.0)
    }
}

fn changed_row(transcript: &Transcript, now: Instant) -> usize {
    (0..transcript.end_row())
        .find(|&row| transcript.file_at(row, now).is_some())
        .unwrap()
        + 1
}

#[test]
fn draws_a_change_under_its_tool_and_the_old_line_under_the_pointer() {
    let start = Instant::now();
    let transcript = edited_reply(start);
    let later = start + Duration::from_secs(5);
    let row = changed_row(&transcript, later);

    let shown = texts(&lines(&transcript, &tall(None), later, &Words));
    let hovered = texts(&lines(&transcript, &tall(Some(row)), later, &Words));

    let top = (crate::reading::to_f32(crate::reading::rows(row)) * LINE).round();
    assert!(
        shown.contains(&(top, 0, "── 1 │ new".to_owned())),
        "{shown:?}"
    );
    assert!(
        hovered.contains(&(top, 0, "── 1 │ old".to_owned())),
        "{hovered:?}"
    );
    assert!(shown.contains(&((top - LINE).round(), 2, "a.rs ↗".to_owned())));
}

#[test]
fn cuts_a_long_line_of_code_at_the_column() {
    let start = Instant::now();
    let transcript = edited_reply_to(start, &"x".repeat(200));
    let later = start + Duration::from_secs(5);
    let row = changed_row(&transcript, later);

    let shown = texts(&lines(&transcript, &tall(None), later, &Words));

    let top = (crate::reading::to_f32(crate::reading::rows(row)) * LINE).round();
    let (_, _, text) = shown
        .iter()
        .find(|(at, _, _)| (at - top).abs() < 0.5)
        .unwrap();
    assert!(text.ends_with('…'), "{text:?}");
    assert_eq!(text.chars().count(), usize::from(TEXT_INDENT) + 94);
}

#[test]
fn recedes_the_code_of_an_older_exchange_with_its_text() {
    let start = Instant::now();
    let mut transcript = edited_reply(start);
    let later = start + Duration::from_secs(5);
    transcript.end_reply(None, later);
    transcript.append("next", later);
    let row = changed_row(&transcript, later);

    let placed = lines(&transcript, &tall(None), later, &Words);

    let top = (crate::reading::to_f32(crate::reading::rows(row)) * LINE).round();
    let line = placed
        .iter()
        .find(|placed| (placed.top - top).abs() < 0.5)
        .unwrap();
    assert_eq!(line.line.alpha, crate::ink::opacity(0.64));
}

#[test]
fn raises_a_changing_word_from_below_its_row() {
    let start = Instant::now();
    let transcript = edited_reply(start);
    let row = changed_row(&transcript, start);
    let changing = start + Duration::from_millis(70 + 150);

    let placed = lines(&transcript, &tall(None), changing, &Words);

    let top = (crate::reading::to_f32(crate::reading::rows(row)) * LINE).round();
    let rising = placed
        .iter()
        .find(|placed| placed.top > top && placed.top < top + LINE && placed.line.indent > 0)
        .unwrap();
    assert!(rising.line.spans[0].heat > 0);
    assert!(rising.line.alpha < OPAQUE);
}
