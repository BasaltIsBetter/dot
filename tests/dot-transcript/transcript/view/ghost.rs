use super::*;
use crate::code::tests::Words;
use crate::ink::{Ink, OPAQUE};
use crate::transcript::Kind;
use crate::transcript::lines;
use crate::transcript::view::tests::{
    LINE, TRAVEL, after, alphas, asked, inks, row_top, texts, view,
};

#[test]
fn draws_the_question_where_the_reply_will_be_written() {
    let start = Instant::now();

    let transcript = asked("hello you", start);

    let placed = lines(&transcript, &view(0.0), after(start, 600), &Words);
    assert_eq!(texts(&placed), vec![(100.0, 0, "› hello you".to_owned())]);
    assert!(placed.iter().all(|placed| placed.clip.is_none()));
}

#[test]
fn a_reached_word_leaves_upward_fading_as_the_reply_rises_in() {
    let start = Instant::now();
    let mut transcript = asked("hello you", start);
    let written = after(start, 600);

    transcript.hear(Kind::Answer, "Hey", written);

    let placed = lines(&transcript, &view(0.0), after(written, 150), &Words);
    let eased = 0.684_643_2_f32;
    let rising = (row_top(0.0) + (1.0 - eased) * TRAVEL * LINE).round();
    let leaving = (row_top(0.0) - eased * TRAVEL * LINE).round();
    assert_eq!(
        texts(&placed),
        vec![
            (rising, 2, "Hey".to_owned()),
            (leaving, 0, "› ".to_owned()),
            (leaving, 2, "hello".to_owned()),
            (100.0, 8, "you".to_owned()),
        ]
    );
    assert!(placed.iter().all(|placed| placed.clip.is_none()));
    let shown = alphas(&placed);
    assert!(shown[0] > 150 && shown[0] < 200, "{shown:?}");
    assert!(shown[1] > 60 && shown[1] < 100, "{shown:?}");
    assert_eq!(shown[1], shown[2]);
    assert_eq!(shown[3], OPAQUE);
}

#[test]
fn the_images_of_the_question_wait_in_secondary_ink_below_it() {
    let start = Instant::now();
    let mut transcript = Transcript::default();
    transcript.append("look", start);
    transcript.attach("image  a.png  2 KB", start);
    transcript.begin_reply(start);

    let placed = lines(&transcript, &view(0.0), after(start, 2_000), &Words);

    assert_eq!(
        texts(&placed),
        [
            (100.0, 0, "› look".to_owned()),
            (row_top(1.0), 2, "image  a.png  2 KB".to_owned()),
        ]
    );
    assert_eq!(inks(&placed[1]), [Ink::Secondary]);
}

#[test]
fn draws_no_ghost_once_the_question_has_gone() {
    let start = Instant::now();
    let mut transcript = asked("hi", start);

    transcript.hear(Kind::Answer, "Hello", after(start, 600));

    let placed = lines(&transcript, &view(2.0), after(start, 1_300), &Words);
    assert_eq!(texts(&placed), vec![(100.0, 2, "Hello_".to_owned())]);
}

#[test]
fn draws_only_the_rows_of_a_long_question_inside_the_band() {
    let start = Instant::now();

    let transcript = asked("1\n2\n3\n4\n5\n6\n7\n8", start);

    let placed = lines(&transcript, &view(0.0), after(start, 2_000), &Words);
    let ghost: Vec<_> = texts(&placed)
        .into_iter()
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
