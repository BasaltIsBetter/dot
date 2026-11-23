use proptest::prelude::*;

use super::*;
use crate::note::push_line;

fn marked(text: &str, fewest: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let run_at = |at: usize| {
        lines[at..]
            .iter()
            .take_while(|line| line.starts_with('x'))
            .count()
    };
    thin(text, &lines, fewest, run_at, |thinner, run| {
        push_line(thinner, &format!("<{} x>", run.len()));
    })
}

#[test]
fn rewrites_the_runs_that_are_long_enough_and_copies_the_rest_byte_for_byte() {
    let text = "a\r\nx1\nx2\nb\tb\r\nx3\nx4\nx5\n\nlast";

    assert_eq!(
        marked(text, 2).unwrap(),
        "a\r\n<2 x>\nb\tb\r\n<3 x>\n\nlast"
    );
    assert_eq!(
        marked(text, 3).unwrap(),
        "a\r\nx1\nx2\nb\tb\r\n<3 x>\n\nlast"
    );
}

#[test]
fn returns_nothing_when_no_run_is_long_enough() {
    assert_eq!(marked("a\nx1\nx2\nb\n", 3), None);
    assert_eq!(marked("a\nb\n", 1), None);
    assert_eq!(marked("", 1), None);
}

#[test]
fn ends_the_text_as_the_original_ends_when_the_last_run_is_rewritten() {
    assert_eq!(marked("a\nx1\nx2", 2).unwrap(), "a\n<2 x>");
    assert_eq!(marked("a\nx1\nx2\n", 2).unwrap(), "a\n<2 x>\n");
    assert_eq!(marked("x1\nx2\nb", 2).unwrap(), "<2 x>\nb");
}

#[test]
fn asks_once_about_each_line_of_a_run_too_short_to_rewrite() {
    let asked = std::cell::Cell::new(0);
    let text = "x\n".repeat(1_000);
    let lines: Vec<&str> = text.lines().collect();
    let run_at = |at: usize| {
        asked.set(asked.get() + 1);
        lines.len() - at
    };

    let thinner = thin(&text, &lines, 2_000, run_at, |_, _| {});

    assert_eq!(thinner, None);
    assert_eq!(asked.get(), 1);
}

proptest! {
    #[test]
    fn keeps_every_line_outside_the_runs_it_rewrites(
        lines in prop::collection::vec("(x|y)[a-z]{0,3}", 0..40),
        fewest in 1_usize..5,
    ) {
        let text = lines.join("\n");

        let Some(thinner) = marked(&text, fewest) else {
            let longest = lines.split(|line| !line.starts_with('x')).map(<[String]>::len).max();
            prop_assert!(longest.unwrap_or(0) < fewest);
            return Ok(());
        };

        let kept: Vec<&str> = thinner.lines().filter(|line| !line.starts_with('<')).collect();
        let expected: Vec<&str> = lines
            .split(|line| !line.starts_with('x'))
            .scan(0, |at, run| {
                let start = *at;
                *at += run.len() + 1;
                Some((start, run))
            })
            .flat_map(|(start, run)| {
                let after = lines.get(start + run.len()).map(String::as_str);
                let run = if run.len() >= fewest { &[] } else { run };
                run.iter().map(String::as_str).chain(after)
            })
            .collect();
        prop_assert_eq!(kept, expected);
    }
}
