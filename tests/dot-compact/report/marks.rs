use proptest::prelude::*;

use super::*;

fn marks(command: &str, text: &str) -> Option<String> {
    compact(&CommandLine::read(command), text)
}

#[test]
fn counts_the_marks_of_a_run_of_examples() {
    let output = format!(
        "Randomized with seed 4242\n\n{}F*\n\nFinished in 1.2 seconds\n120 examples, 1 failure, 1 pending\n",
        ".".repeat(118)
    );

    assert_eq!(
        marks("bundle exec rspec", &output).as_deref(),
        Some(
            "Randomized with seed 4242\n\n[+120 marks: 118 . 1 F 1 *]\n\nFinished in 1.2 seconds\n120 examples, 1 failure, 1 pending\n"
        )
    );
}

#[test]
fn counts_the_files_rubocop_inspected() {
    let output = "Inspecting 24 files\n  .....C..........W.......\n\n24 files inspected, 2 offenses detected";

    assert_eq!(
        marks("rubocop", output).as_deref(),
        Some(
            "Inspecting 24 files\n[+24 marks: 22 . 1 C 1 W]\n\n24 files inspected, 2 offenses detected"
        )
    );
}

#[test]
fn needs_twenty_marks_of_which_three_in_four_passed() {
    let dots = |count: usize| ".".repeat(count);

    assert_eq!(marks("rspec", &dots(19)), None);
    assert_eq!(
        marks("rspec", &dots(20)).as_deref(),
        Some("[+20 marks: 20 .]")
    );
    assert_eq!(marks("rspec", &format!("{}FFFFFF", dots(14))), None);
    assert_eq!(
        marks("rspec", &format!("{}FFFFF", dots(15))).as_deref(),
        Some("[+20 marks: 15 . 5 F]")
    );
    assert_eq!(marks("rspec", &"F".repeat(40)), None);
}

#[test]
fn a_line_with_anything_else_is_not_a_line_of_marks() {
    for line in [
        "..........................x",
        "............. .............",
        "..........................1",
        "...........(pending).......",
        "see https://example.com/...",
    ] {
        assert_eq!(marks("rspec", line), None, "{line}");
    }
}

#[test]
fn only_reads_the_tools_that_draw_marks() {
    let dots = ".".repeat(40);

    for command in ["rspec", "rake test", "mix test", "bundle exec rubocop"] {
        assert!(marks(command, &dots).is_some(), "{command}");
    }
    assert_eq!(marks("cargo test", &dots), None);
    assert_eq!(marks("cat dots.txt", &dots), None);
}

proptest! {
    #[test]
    fn counts_every_mark_once(line in "[.*FE]{0,80}") {
        if let Some(counted) = marks("rspec", &line) {
            let counts: usize = counted
                .trim_end_matches(']')
                .split(' ')
                .skip(2)
                .step_by(2)
                .map(|count| count.parse::<usize>().unwrap())
                .sum();
            let total = line.chars().count();
            let opening = format!("[+{total} marks:");
            prop_assert_eq!(counts, total);
            prop_assert!(counted.starts_with(&opening));
        }
    }
}
