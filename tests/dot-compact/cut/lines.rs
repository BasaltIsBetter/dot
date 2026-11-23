use super::*;
use crate::cut::{CONTENT, OTHER};

#[test]
fn cuts_the_end_of_a_long_line_and_counts_it() {
    let text = format!("short\n{}\nlast", "x".repeat(500));

    assert_eq!(
        shorten(&text, &OTHER),
        Some(format!("short\n{}[+100 chars]\nlast", "x".repeat(400)))
    );
}

#[test]
fn leaves_alone_a_line_that_would_lose_less_than_forty_bytes() {
    assert_eq!(shorten(&"x".repeat(439), &OTHER), None);
    assert_eq!(
        shorten(&"x".repeat(440), &OTHER),
        Some(format!("{}[+40 chars]", "x".repeat(400)))
    );
    assert_eq!(shorten("short\nlines\n", &OTHER), None);
    assert_eq!(shorten("", &OTHER), None);
}

#[test]
fn cuts_between_two_characters_and_counts_characters_not_bytes() {
    let euros = "€".repeat(200);

    assert_eq!(
        shorten(&euros, &OTHER),
        Some(format!("{}[+67 chars]", "€".repeat(133)))
    );
}

#[test]
fn spares_the_prose_of_a_file_and_cuts_its_data() {
    let prose = "word ".repeat(400);
    let data = "A".repeat(2_000);
    let one_space_in_twelve = "elevenchars ".repeat(200);
    let fewer_spaces = format!("{}x", &one_space_in_twelve[..2_399]);

    assert_eq!(shorten(&prose, &CONTENT), None);
    assert_eq!(shorten(&one_space_in_twelve, &CONTENT), None);
    assert_eq!(
        shorten(&data, &CONTENT),
        Some(format!("{}[+1000 chars]", "A".repeat(1_000)))
    );
    assert_eq!(
        shorten(&fewer_spaces, &CONTENT).map(|cut| cut.len()),
        Some(1_000 + "[+1400 chars]".len())
    );
    assert_eq!(
        shorten(&prose, &OTHER).map(|cut| cut.len()),
        Some(400 + "[+1600 chars]".len())
    );
}

#[test]
fn keeps_every_line_break_where_it_was() {
    let long = "y".repeat(450);
    let text = format!("\n{long}\n\nend\n");

    assert_eq!(
        shorten(&text, &OTHER),
        Some(format!("\n{}[+50 chars]\n\nend\n", "y".repeat(400)))
    );
}
