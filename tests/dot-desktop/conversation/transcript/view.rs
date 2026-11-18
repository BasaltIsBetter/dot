use std::time::Duration;

use winit::dpi::PhysicalSize;

use super::*;

const BAND: Band = Band {
    top: 100.0,
    bottom: 260.0,
};

fn layout() -> Layout {
    Layout::new(PhysicalSize::new(2880, 1800), 2.0)
}

fn texts(placed: &[Placed]) -> Vec<(f32, u16, String)> {
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
    transcript.append("first".to_owned(), sent);
    transcript.append("second\nline".to_owned(), sent);

    let placed = lines(&transcript, BAND, &layout(), sent + Duration::from_secs(2));

    assert_eq!(
        texts(&placed),
        vec![
            (100.0, 0, "› first".to_owned()),
            (180.0, 0, "› second".to_owned()),
            (220.0, 2, "line".to_owned()),
        ]
    );
}

#[test]
fn settled_entries_have_a_receding_marker_and_full_ink_text() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello".to_owned(), sent);

    let placed = lines(&transcript, BAND, &layout(), sent + Duration::from_secs(2));

    let spans = &placed[0].line.spans;
    assert_eq!((spans[0].ink, spans[0].heat), (Ink::Receding, 0));
    assert_eq!((spans[1].ink, spans[1].heat), (Ink::Full, 0));
}

#[test]
fn a_new_entry_arrives_orange_and_cools() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("hello\nworld".to_owned(), sent);

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
        transcript.append(format!("entry {number}"), sent);
    }

    let placed = lines(&transcript, BAND, &layout(), sent);

    assert_eq!(
        texts(&placed)
            .into_iter()
            .map(|(top, _, text)| (top, text))
            .collect::<Vec<_>>(),
        vec![
            (140.0, "› entry 98".to_owned()),
            (220.0, "› entry 99".to_owned()),
        ]
    );
}

#[test]
fn places_rows_between_pixels_when_scrolled_by_a_fraction() {
    let sent = Instant::now();
    let mut transcript = Transcript::default();
    for number in 0..10 {
        transcript.append(format!("entry {number}"), sent);
    }
    transcript.scroll_by(0.5, BAND.rows(40.0));

    let placed = lines(&transcript, BAND, &layout(), sent);

    assert_eq!(placed[0].top, 80.0);
    assert_eq!(texts(&placed)[0].2, "› entry 7");
}
