use super::*;
use crate::canvas::Ink;
use crate::conversation::transcript::Kind;
use crate::conversation::transcript::lines;
use crate::conversation::transcript::view::tests::{
    BAND, LINE, after, asked, inks, layout, row_top, texts,
};

#[test]
fn draws_the_question_where_the_reply_will_be_written() {
    let start = Instant::now();

    let transcript = asked("hello you", start);

    let placed = lines(&transcript, BAND, &layout(), after(start, 600));
    assert_eq!(
        texts(&placed),
        vec![
            (100.0, 2, "_".to_owned()),
            (100.0, 0, "› hello you".to_owned()),
        ]
    );
    assert!(placed.iter().all(|placed| placed.clip.is_none()));
}

#[test]
fn a_reached_word_leaves_through_the_top_of_its_row_as_the_reply_rises_in() {
    let start = Instant::now();
    let mut transcript = asked("hello you", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hey", BAND.rows(LINE), written);

    let placed = lines(&transcript, BAND, &layout(), after(written, 150));
    let row = Some([row_top(0.0), BAND.top + LINE]);
    let rising = row_top(1.0 - 0.684_643_2);
    let leaving = row_top(-0.684_643_2);
    assert_eq!(
        texts(&placed),
        vec![
            (rising, 2, "Hey_".to_owned()),
            (leaving, 0, "› ".to_owned()),
            (leaving, 2, "hello".to_owned()),
            (100.0, 8, "you".to_owned()),
        ]
    );
    assert_eq!(
        placed.iter().map(|placed| placed.clip).collect::<Vec<_>>(),
        vec![row, row, row, None]
    );
}

#[test]
fn the_images_of_the_question_wait_in_secondary_ink_below_it() {
    let start = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("look", start);
    transcript.attach("image  a.png  2 KB", start);
    transcript.begin_reply(BAND.rows(LINE), start);

    let placed = lines(&transcript, BAND, &layout(), after(start, 2_000));

    assert_eq!(
        texts(&placed)[1..],
        [
            (100.0, 0, "› look".to_owned()),
            (row_top(1.0), 2, "image  a.png  2 KB".to_owned()),
        ]
    );
    assert_eq!(inks(&placed[2]), [Ink::Secondary]);
}

#[test]
fn draws_no_ghost_once_the_question_has_gone() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);

    transcript.hear(Kind::Answer, "Hello", BAND.rows(LINE), after(start, 600));

    let placed = lines(&transcript, BAND, &layout(), after(start, 900));
    assert_eq!(texts(&placed), vec![(100.0, 2, "Hello_".to_owned())]);
}

#[test]
fn draws_only_the_rows_of_a_long_question_inside_the_band() {
    let start = Instant::now();

    let transcript = asked("1\n2\n3\n4\n5\n6\n7\n8", start);

    let placed = lines(&transcript, BAND, &layout(), after(start, 2_000));
    let ghost: Vec<_> = texts(&placed)
        .into_iter()
        .skip(1)
        .map(|(top, _, text)| (top, text))
        .collect();
    assert_eq!(
        ghost,
        vec![
            (row_top(0.0), "› 1".to_owned()),
            (row_top(1.0), "2".to_owned()),
            (row_top(2.0), "3".to_owned()),
            (row_top(3.0), "4".to_owned()),
        ]
    );
}
