use proptest::prelude::*;

use super::*;

fn numbered(lines: usize) -> String {
    (1..=lines)
        .map(|number| format!("line {number:04} of the output\n"))
        .collect::<Vec<String>>()
        .concat()
}

#[test]
fn ends_an_output_it_cut_with_the_way_to_read_it_whole() {
    let with_a_last_line_break = cut(&numbered(300), &OTHER).unwrap();
    let without = cut(numbered(300).trim_end(), &OTHER).unwrap();

    assert!(
        with_a_last_line_break.ends_with(&format!("line 0300 of the output\n{CUT_OUTPUT_NOTE}\n"))
    );
    assert!(without.ends_with(&format!("line 0300 of the output\n{CUT_OUTPUT_NOTE}")));
}

#[test]
fn cuts_nothing_from_an_output_that_is_short_enough_even_its_long_lines() {
    let long_line = format!("{}\n", "x".repeat(4_990));

    assert_eq!(cut(&long_line, &OTHER), None);
    assert_eq!(cut(&numbered(200), &OTHER), None);
}

#[test]
fn cuts_the_long_lines_first_and_the_middle_only_if_it_is_still_too_long() {
    let three_long_lines = format!("{0}\n{0}\n{0}\n", "x".repeat(3_000));
    let cut_line = format!("{}[+2600 chars]\n", "x".repeat(400));

    assert_eq!(
        cut(&three_long_lines, &OTHER),
        Some(format!("{cut_line}{cut_line}{cut_line}{CUT_OUTPUT_NOTE}\n"))
    );
}

#[test]
fn keeps_the_start_of_a_document_written_on_one_line() {
    let document = format!("{{\"items\":[{}]}}", "1,".repeat(4_000));

    assert_eq!(
        cut(&document, &ONE_DOCUMENT),
        Some(format!(
            "{}[+5012 chars]\n{CUT_OUTPUT_NOTE}",
            &document[..3_000]
        ))
    );
    assert_eq!(cut(&document[..5_000], &ONE_DOCUMENT), None);
}

proptest! {
    #[test]
    fn what_is_kept_is_the_original_in_its_order(
        lines in prop::collection::vec("[a-z ]{40,120}", 0..120),
    ) {
        let text = lines.join("\n");

        match cut(&text, &OTHER) {
            None => prop_assert!(text.len() <= 7_100),
            Some(shorter) => {
                let mut original = text.lines();
                let kept = shorter.lines().filter(|line| !line.starts_with('['));

                prop_assert!(shorter.len() < text.len());
                prop_assert!(shorter.trim_end_matches(NEW_LINE).ends_with(CUT_OUTPUT_NOTE));
                prop_assert_eq!(shorter.ends_with(NEW_LINE), text.ends_with(NEW_LINE));
                for line in kept {
                    prop_assert!(original.any(|written| written == line), "{line}");
                }
            }
        }
    }

    #[test]
    fn any_text_is_cut_without_panicking(text in any::<String>(), repeats in 1_usize..60) {
        let long = text.repeat(repeats);

        for limits in [&CONTENT, &OTHER, &ONE_DOCUMENT] {
            if let Some(shorter) = cut(&long, limits) {
                prop_assert!(shorter.len() < long.len());
            }
        }
    }
}

fn in_capitals(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_uppercase())
}

fn without_3_999_bytes(text: &str) -> Option<String> {
    text.get(3_999..).map(str::to_owned)
}

fn without_4_000_bytes(text: &str) -> Option<String> {
    text.get(4_000..).map(str::to_owned)
}

const IN_CAPITALS: &[thin::Thinning] = &[in_capitals];
const SHORT_OF_3_999: &[thin::Thinning] = &[without_3_999_bytes];
const SHORT_OF_4_000: &[thin::Thinning] = &[without_4_000_bytes];
const IN_CAPITALS_THEN_SHORTER: &[thin::Thinning] = &[in_capitals, without_4_000_bytes];

const fn thinned_by(thinnings: &'static [thin::Thinning]) -> Limits {
    Limits {
        longest_line: 100_000,
        spares_prose: false,
        cut_above: 10,
        kept: 8,
        kept_from_the_start: 4,
        outlined: false,
        thinnings,
    }
}

#[test]
fn keeps_a_way_of_cutting_only_when_it_makes_the_output_shorter() {
    let text = "x".repeat(6_000);

    assert_eq!(cut(&text, &thinned_by(IN_CAPITALS)), None);
    assert_eq!(
        cut(&text, &thinned_by(IN_CAPITALS_THEN_SHORTER)),
        Some(format!("{}\n{CUT_OUTPUT_NOTE}", "x".repeat(2_000)))
    );
}

#[test]
fn cuts_only_when_that_removes_4_000_bytes() {
    let text = "x".repeat(6_000);

    assert_eq!(cut(&text, &thinned_by(SHORT_OF_3_999)), None);
    assert_eq!(
        cut(&text, &thinned_by(SHORT_OF_4_000)),
        Some(format!("{}\n{CUT_OUTPUT_NOTE}", "x".repeat(2_000)))
    );
}

#[test]
fn cuts_an_output_above_7_000_bytes_and_the_contents_of_a_file_above_10_000() {
    assert_eq!(OTHER.cut_above, 7_000);
    assert_eq!(OTHER.kept, 3_000);
    assert_eq!(CONTENT.cut_above, 10_000);
    assert_eq!(CONTENT.kept, 6_000);
    assert_eq!(ONE_DOCUMENT.cut_above, 7_000);
    assert_eq!(OTHER.thinnings.len(), 7);
    assert_eq!(CONTENT.thinnings.len(), 1);
    assert_eq!(ONE_DOCUMENT.thinnings.len(), 0);
}

#[test]
fn keeps_two_thirds_of_a_file_from_its_start_and_half_of_another_output() {
    let bounds = |above, kept| dot_api::CutBounds { above, kept };

    let file = Limits::of_contents(bounds(13_000, 9_000));
    assert_eq!(
        (file.cut_above, file.kept, file.kept_from_the_start),
        (13_000, 9_000, 6_000)
    );
    let output = Limits::of_outputs(bounds(6_000, 2_000));
    assert_eq!(
        (output.cut_above, output.kept, output.kept_from_the_start),
        (6_000, 2_000, 1_000)
    );
    let document = Limits::of_a_document(bounds(9_000, 5_000));
    assert_eq!((document.longest_line, document.cut_above), (5_000, 9_000));
    assert!(document.thinnings.is_empty() && !document.outlined);
    assert_eq!(CONTENT.kept_from_the_start, 4_000);
    assert_eq!(OTHER.kept_from_the_start, 1_500);
}
