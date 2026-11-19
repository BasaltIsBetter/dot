use std::time::Duration;

use winit::dpi::PhysicalSize;

use super::*;
use crate::conversation::transcript::entry::Kind;

pub(super) const LINE: f32 = 20.0 * 2.0 * 0.9;
pub(super) const BAND: Band = Band {
    top: 100.0,
    bottom: 100.0 + 4.0 * LINE,
};

pub(super) fn row_top(rows_down: f32) -> f32 {
    (BAND.top + rows_down * LINE).round()
}

pub(super) fn layout() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), 2.0)
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

#[test]
fn shows_each_entry_with_its_marker_and_a_blank_row_between() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("first", sent);
    transcript.append("second\nline", sent);

    let placed = lines(&transcript, BAND, &layout(), sent + Duration::from_secs(2));

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

    let placed = lines(&transcript, BAND, &layout(), sent + Duration::from_secs(2));

    let spans = &placed[0].line.spans;
    assert_eq!((spans[0].ink, spans[0].heat), (Ink::Receding, 0));
    assert_eq!((spans[1].ink, spans[1].heat), (Ink::Full, 0));
}

#[test]
fn a_new_entry_arrives_orange_and_cools() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello\nworld", sent);

    let arriving = lines(&transcript, BAND, &layout(), sent);
    let cooling = lines(
        &transcript,
        BAND,
        &layout(),
        sent + Duration::from_millis(600),
    );

    assert!(
        arriving
            .iter()
            .flat_map(|placed| &placed.line.spans)
            .all(|span| span.heat == u8::MAX)
    );
    assert!(
        cooling
            .iter()
            .flat_map(|placed| &placed.line.spans)
            .all(|span| span.heat == 80)
    );
}

#[test]
fn draws_only_the_rows_inside_the_band() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..100 {
        transcript.append(&format!("entry {number}"), sent);
    }

    let placed = lines(&transcript, BAND, &layout(), sent);

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
fn places_rows_between_pixels_when_scrolled_by_a_fraction() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..10 {
        transcript.append(&format!("entry {number}"), sent);
    }
    transcript.scroll_by(0.5, BAND.rows(LINE), sent);

    let placed = lines(&transcript, BAND, &layout(), sent);

    assert_eq!(placed[0].top, row_top(-0.5));
    assert_eq!(texts(&placed)[0].2, "› entry 7");
}

pub(super) fn inks(placed: &Placed) -> Vec<Ink> {
    placed.line.spans.iter().map(|span| span.ink).collect()
}

#[test]
fn draws_an_orange_caret_after_the_reply_while_claude_writes() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    transcript.begin_reply(BAND.rows(LINE), now);
    let waiting = lines(&transcript, BAND, &layout(), now);

    transcript.hear(Kind::Answer, "Hi", BAND.rows(LINE), now);
    let streaming = lines(&transcript, BAND, &layout(), now);
    transcript.end_reply(None, BAND.rows(LINE), now);
    let finished = lines(&transcript, BAND, &layout(), now);

    assert_eq!(texts(&waiting).last().unwrap().1, 2);
    assert_eq!(texts(&waiting).last().unwrap().2, "_");
    assert_eq!(texts(&streaming).last().unwrap().2, "Hi_");
    assert_eq!(inks(streaming.last().unwrap()).last(), Some(&Ink::Live));
    assert_eq!(texts(&finished).last().unwrap().2, "Hi");
}

#[test]
fn writes_thinking_in_secondary_ink_and_notices_receding() {
    let now = Instant::now();
    let mut transcript = Transcript::default();
    let tall = Band {
        top: 100.0,
        bottom: 500.0,
    };
    transcript.append("hello", now);
    transcript.begin_reply(tall.rows(LINE), now);
    transcript.hear(Kind::Thinking, "Hm.", tall.rows(LINE), now);
    transcript.hear(Kind::Answer, "Hi", tall.rows(LINE), now);
    transcript.end_reply(Some("reply-stopped"), tall.rows(LINE), now);

    let placed = lines(&transcript, tall, &layout(), now + Duration::from_secs(2));

    assert_eq!(placed.len(), 3);
    assert_eq!(inks(&placed[0]), [Ink::Secondary]);
    assert_eq!(inks(&placed[1]), [Ink::Full]);
    assert_eq!(inks(&placed[2]), [Ink::Receding]);
    assert!(placed.iter().all(|placed| placed.line.indent == 2));
}

#[test]
fn heats_each_piece_by_its_own_arrival() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hi", sent);
    transcript.begin_reply(BAND.rows(LINE), sent);
    transcript.hear(Kind::Answer, "old ", BAND.rows(LINE), sent);
    transcript.hear(
        Kind::Answer,
        "new",
        BAND.rows(LINE),
        sent + Duration::from_millis(600),
    );

    let placed = lines(
        &transcript,
        BAND,
        &layout(),
        sent + Duration::from_millis(600),
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
    transcript.begin_reply(BAND.rows(LINE), start);
    transcript
}

#[test]
fn the_reply_settles_in_place_and_the_reached_words_are_gone() {
    let start = Instant::now();
    let mut transcript = asked("hello you", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hey", BAND.rows(LINE), written);

    let placed = lines(&transcript, BAND, &layout(), after(written, 300));
    assert_eq!(
        texts(&placed),
        vec![(100.0, 2, "Hey_".to_owned()), (100.0, 8, "you".to_owned())]
    );
}

#[test]
fn the_reply_rises_only_while_it_replaces_the_question() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hello there", BAND.rows(LINE), written);
    transcript.hear(Kind::Answer, " friend", BAND.rows(LINE), written);

    let placed = lines(&transcript, BAND, &layout(), after(written, 100));
    let replies = &texts(&placed)[..2];
    assert!(replies[0].0 > 100.0);
    assert_eq!((replies[0].1, replies[0].2.as_str()), (2, "Hello there"));
    assert_eq!(replies[1], (100.0, 13, " friend_".to_owned()));
}
