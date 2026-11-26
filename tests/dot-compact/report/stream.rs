use proptest::prelude::*;

use super::*;
use crate::report::progress::compact;

fn stream(command: &str) -> LineStream {
    LineStream::of(&CommandLine::read(command), Toggle::On, Toggle::On)
}

#[test]
fn a_command_that_names_no_known_tool_only_loses_its_noise() {
    let mut stream = stream("python3 train.py");

    assert_eq!(
        stream.push("   Compiling a v1\n   Compiling b v1\n   Compiling c v1\n"),
        "   Compiling a v1\n   Compiling b v1\n   Compiling c v1\n"
    );
    assert_eq!(stream.push("\u{1b}[32mepoch 1\u{1b}[0m\n"), "epoch 1\n");
    assert_eq!(stream.rest(), "");
}

#[test]
fn tells_a_line_redrawn_in_place_once_as_it_was_drawn_last() {
    let mut stream = stream("python3 train.py");

    assert_eq!(stream.push("10%\r20%\r"), "");
    assert_eq!(stream.push("30%\r40%"), "");
    assert_eq!(stream.push("\rdone\nnext"), "done\n");
    assert_eq!(stream.rest(), "next");
}

#[test]
fn forgets_what_was_drawn_over_instead_of_keeping_it_waiting() {
    let mut stream = stream("cargo build");
    let frames = "progress 50%\r".repeat(10_000);
    assert!(frames.len() > LONGEST_WAIT);

    assert_eq!(stream.push(&frames), "");
    assert_eq!(stream.rest(), "progress 50%\r");
}

#[test]
fn a_windows_line_ending_cut_between_two_pieces_is_still_one() {
    let mut stream = stream("python3 train.py");

    assert_eq!(stream.push("first\r"), "");
    assert_eq!(stream.push("\nsecond\r\n"), "first\nsecond\n");
}

#[test]
fn tells_a_line_as_soon_as_nothing_can_change_it() {
    let mut stream = stream("cargo test");

    assert_eq!(
        stream.push("running 3 tests\ntest a ... ok\n"),
        "running 3 tests\n"
    );
    assert_eq!(stream.push("test b ... ok\ntest c ... ok\n"), "");
    assert_eq!(
        stream.push("test result: ok. 3 passed\n"),
        "[+3 lines: tests that passed]\ntest result: ok. 3 passed\n"
    );
    assert_eq!(stream.rest(), "");
}

#[test]
fn counts_a_run_still_open_when_the_output_pauses() {
    let mut stream = stream("cargo build");

    assert_eq!(
        stream.push("   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n"),
        ""
    );
    assert_eq!(stream.rest(), "[+3 lines: build progress]\n");
    assert_eq!(stream.rest(), "");
    assert_eq!(stream.push("    Finished in 2s\n"), "    Finished in 2s\n");
}

#[test]
fn keeps_a_run_too_short_to_be_worth_a_count() {
    let mut stream = stream("cargo build");

    stream.push("   Compiling a v1.0.0\n   Compiling b v1.0.0\n");

    assert_eq!(
        stream.rest(),
        "   Compiling a v1.0.0\n   Compiling b v1.0.0\n"
    );
}

#[test]
fn waits_for_the_end_of_a_line_before_reading_it() {
    let mut stream = stream("cargo build");

    assert_eq!(stream.push("warning: unus"), "");
    assert_eq!(
        stream.push("ed variable\n   Comp"),
        "warning: unused variable\n"
    );
    assert_eq!(stream.rest(), "   Comp");
}

#[test]
fn removes_the_colours_of_the_lines_it_tells() {
    let mut stream = stream("cargo test");

    assert_eq!(
        stream.push("\u{1b}[32mtest result: ok\u{1b}[0m\n"),
        "test result: ok\n"
    );
}

#[test]
fn reads_a_task_again_once_it_knows_what_follows_it() {
    let mut stream = stream("./gradlew build");

    assert_eq!(stream.push("> Task :a\n> Task :b\n> Task :c\n"), "");
    assert_eq!(
        stream.push("warning: deprecated\n"),
        "> Task :a\n> Task :b\n> Task :c\nwarning: deprecated\n"
    );
    assert_eq!(
        stream.push("> Task :d\n> Task :e\n> Task :f\n\nBUILD SUCCESSFUL in 2s\n"),
        "[+3 lines: tasks with nothing to report]\n\nBUILD SUCCESSFUL in 2s\n"
    );
}

#[test]
fn never_keeps_more_than_64_kibibytes_waiting() {
    let mut stream = stream("cargo build");
    let lines = (0..3_000)
        .map(|krate| format!("   Compiling crate_{krate} v1.0.0\n"))
        .collect::<Vec<String>>()
        .concat();
    assert!(lines.len() > LONGEST_WAIT);

    assert_eq!(stream.push(&lines), "[+3000 lines: build progress]\n");
    assert_eq!(stream.rest(), "");
}

#[test]
fn keeps_waiting_up_to_exactly_64_kibibytes() {
    let mut stream = stream("cargo build");
    let run = (0..100)
        .map(|krate| format!("   Compiling crate_{krate} v1.0.0\n"))
        .collect::<Vec<String>>()
        .concat();
    assert!(run.len() > 2_000 && run.len() < LONGEST_WAIT);

    assert_eq!(stream.push(&run), "");
    assert_eq!(stream.rest(), "[+100 lines: build progress]\n");
    assert_eq!(LONGEST_WAIT, 65_536);
    assert_eq!(stream.push(&"x".repeat(LONGEST_WAIT)), "");
    assert_eq!(stream.push("y").len(), LONGEST_WAIT + 1);
}

#[test]
fn a_long_text_without_a_line_end_is_told_as_it_is() {
    let mut stream = stream("cargo build");
    let long = "x".repeat(LONGEST_WAIT + 1);

    assert_eq!(stream.push(&long), long);
}

proptest! {
    #[test]
    fn says_the_same_whatever_the_pieces_it_reads(
        lines in prop::collection::vec(prop_oneof![
            Just("   Compiling a v1.0.0\n"),
            Just("test a ... ok\n"),
            Just("warning: unused\n"),
            Just("\n"),
            Just("test result: ok. 1 passed\n"),
        ], 0..60),
        cuts in prop::collection::vec(1_usize..40, 0..30),
    ) {
        let text = lines.concat();
        let command = CommandLine::read("cargo test");
        let whole = compact(&command, &text).unwrap_or_else(|| text.clone());
        let mut stream = LineStream::of(&command, Toggle::On, Toggle::On);
        let mut told = String::new();
        let mut rest = text.as_str();

        for cut in cuts {
            let (piece, after) = rest.split_at(cut.min(rest.len()));
            told.push_str(&stream.push(piece));
            rest = after;
        }
        told.push_str(&stream.push(rest));
        told.push_str(&stream.rest());

        prop_assert_eq!(told, whole);
    }

    #[test]
    fn redrawn_lines_are_told_the_same_whatever_the_pieces_it_reads(
        lines in prop::collection::vec(prop_oneof![
            Just("10%\r50%\r100%\n"),
            Just("\u{1b}[32mok\u{1b}[0m\n"),
            Just("plain\r\n"),
            Just("warning: unused\n"),
            Just("\r\n"),
        ], 0..40),
        cuts in prop::collection::vec(1_usize..12, 0..40),
    ) {
        let text = lines.concat();
        let mut stream = LineStream::of(&CommandLine::read("python3 train.py"), Toggle::On, Toggle::On);
        let mut told = String::new();
        let mut rest = text.as_str();

        for cut in cuts {
            let (piece, after) = rest.split_at(cut.min(rest.len()));
            told.push_str(&stream.push(piece));
            rest = after;
        }
        told.push_str(&stream.push(rest));
        told.push_str(&stream.rest());

        prop_assert_eq!(told, noise::clean(&text));
    }

    #[test]
    fn any_piece_is_read_without_panicking(pieces in prop::collection::vec(any::<String>(), 0..8)) {
        let mut stream = LineStream::of(&CommandLine::read("cargo test"), Toggle::On, Toggle::On);
        let read: usize = pieces.iter().map(String::len).sum();
        let mut told = 0;

        for piece in &pieces {
            told += stream.push(piece).len();
        }
        told += stream.rest().len();

        prop_assert!(told <= read + pieces.len() * 40);
    }
}

#[test]
fn keeps_waiting_the_line_it_has_not_finished_when_finished_lines_pass_64_kibibytes() {
    let mut stream = stream("python3 train.py");
    let epochs = (0..8_000)
        .map(|epoch| format!("epoch {epoch} done\n"))
        .collect::<Vec<String>>()
        .concat();
    assert!(epochs.len() > LONGEST_WAIT);

    assert_eq!(stream.push(&format!("{epochs}epoch 8000 do")), epochs);
    assert_eq!(stream.push("ne\n"), "epoch 8000 done\n");
    assert_eq!(stream.rest(), "");
}

#[test]
fn tells_a_run_past_64_kibibytes_and_keeps_the_line_after_it_waiting() {
    let mut stream = stream("cargo build");
    let lines = (0..3_000)
        .map(|krate| format!("   Compiling crate_{krate} v1.0.0\n"))
        .collect::<Vec<String>>()
        .concat();

    assert_eq!(
        stream.push(&format!("{lines}   Comp")),
        "[+3000 lines: build progress]\n"
    );
    assert_eq!(stream.rest(), "   Comp");
}

#[test]
fn keeps_waiting_a_run_and_a_line_of_64_kibibytes_together_to_the_byte() {
    let mut stream = stream("cargo build");
    let run = "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n";
    let unfinished = "x".repeat(LONGEST_WAIT - run.len());

    assert_eq!(stream.push(&format!("{run}{unfinished}")), "");
    assert_eq!(stream.push("y"), "[+3 lines: build progress]\n");
    assert_eq!(stream.rest(), format!("{unfinished}y"));
}

proptest! {
    #[test]
    fn tells_every_line_whole_and_ends_mid_line_only_past_64_kibibytes(
        lengths in prop::collection::vec(
            prop_oneof![19 => 0_usize..120, 1 => 60_000_usize..150_000],
            0..200,
        ),
        unfinished in 0_usize..100_000,
        cuts in prop::collection::vec(1_usize..300_000, 0..16),
    ) {
        let mut text = lengths
            .iter()
            .enumerate()
            .map(|(at, &length)| format!("{at}:{}\n", "a".repeat(length)))
            .collect::<Vec<String>>()
            .concat();
        text.push_str(&"b".repeat(unfinished));
        let mut stream = LineStream::of(&CommandLine::read("python3 train.py"), Toggle::On, Toggle::On);
        let mut told = String::new();
        let mut rest = text.as_str();

        for cut in cuts {
            let (piece, after) = rest.split_at(cut.min(rest.len()));
            let now = stream.push(piece);
            let after_the_last_line = now.rsplit(NEW_LINE).next().unwrap_or_default();
            prop_assert!(after_the_last_line.is_empty() || after_the_last_line.len() > LONGEST_WAIT);
            told.push_str(&now);
            rest = after;
        }
        told.push_str(&stream.push(rest));
        told.push_str(&stream.rest());

        prop_assert_eq!(told, text);
    }
}

struct StreamBefore {
    rules: Vec<&'static Rule>,
    waiting: String,
}

impl StreamBefore {
    fn of(command: &str) -> Self {
        Self {
            rules: active(&CommandLine::read(command)),
            waiting: String::new(),
        }
    }

    fn push(&mut self, piece: &str) -> String {
        self.waiting.push_str(piece);
        let unfinished = self.waiting.rfind(NEW_LINE).map_or(0, |end| end + 1);
        let drawn_over = noise::drawn_over(&self.waiting[unfinished..]);
        self.waiting
            .replace_range(unfinished..unfinished + drawn_over, "");
        let complete_end = self.waiting.rfind(NEW_LINE).map_or(0, |end| end + 1);
        let unfinished = self.waiting.split_off(complete_end);
        let complete = std::mem::replace(&mut self.waiting, unfinished);
        let clean = noise::clean(&complete);
        let lines: Vec<&str> = clean.lines().collect();
        let settled = (0..lines.len())
            .rev()
            .find(|&at| !self.rules.iter().any(|rule| rule.drops(&lines, at)))
            .map_or(0, |last_kept| last_kept + 1);
        let open = &lines[settled..];
        let open_length: usize = open.iter().map(|line| line.len() + 1).sum();
        if open_length + self.waiting.len() > LONGEST_WAIT {
            let mut at_once = told(&self.rules, &lines);
            if self.waiting.len() > LONGEST_WAIT {
                at_once.push_str(&std::mem::take(&mut self.waiting));
            }
            return at_once;
        }
        if !open.is_empty() {
            let mut waiting = open.join("\n");
            waiting.push(NEW_LINE);
            self.waiting.insert_str(0, &waiting);
        }
        told(&self.rules, &lines[..settled])
    }

    fn rest(&mut self) -> String {
        let waiting = std::mem::take(&mut self.waiting);
        let (complete, unfinished) = waiting
            .rfind(NEW_LINE)
            .map_or(("", waiting.as_str()), |end| waiting.split_at(end + 1));
        let clean = noise::clean(complete);
        let lines: Vec<&str> = clean.lines().collect();
        format!("{}{unfinished}", told(&self.rules, &lines))
    }
}

fn piece() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("   Compiling a v1\n".to_owned()),
        Just("   Compiling b".to_owned()),
        Just(" v2\n".to_owned()),
        Just("warning: unused\n".to_owned()),
        Just("\u{1b}[32m   Compiling c v3\u{1b}[0m\n".to_owned()),
        Just("50%\r60%\r".to_owned()),
        Just("\n".to_owned()),
        Just("error[E0425]: x\n".to_owned()),
        "[a-z ]{0,6}\n?",
    ]
}

proptest! {
    #[test]
    fn tells_what_it_told_when_it_cleaned_every_waiting_line_again(
        pieces in proptest::collection::vec(piece(), 0..40),
    ) {
        let mut now = stream("cargo build");
        let mut before = StreamBefore::of("cargo build");

        for piece in &pieces {
            prop_assert_eq!(now.push(piece), before.push(piece));
        }
        prop_assert_eq!(now.rest(), before.rest());
    }
}

#[test]
fn tells_the_same_past_the_longest_wait() {
    let mut now = stream("cargo build");
    let mut before = StreamBefore::of("cargo build");
    let line = "   Compiling a-crate-with-a-long-name v1.0.0 (/Users/someone/projects/a-long-workspace-name/crates/a-crate-with-a-long-name)\n";

    for _ in 0..(LONGEST_WAIT / line.len()) * 3 {
        assert_eq!(now.push(line), before.push(line));
    }
    assert_eq!(now.push("Finished\n"), before.push("Finished\n"));
    assert_eq!(now.rest(), before.rest());
}

#[test]
fn keeps_an_unfinished_line_of_exactly_the_longest_wait_waiting() {
    let mut now = stream("cargo build");
    let unfinished = "x".repeat(LONGEST_WAIT);

    assert_eq!(now.push("   Compiling crate v1.0.0\n"), "");
    let told = now.push(&unfinished);

    assert!(told.len() < LONGEST_WAIT, "{} bytes told", told.len());
    assert_eq!(now.rest(), unfinished);
}

#[test]
fn counts_the_line_break_of_each_waiting_line_toward_the_longest_wait() {
    let line = "   Compiling crate v1.0.0\n";
    let just_over = LONGEST_WAIT / line.len() + 1;
    assert!(just_over * (line.len() - 1) <= LONGEST_WAIT);
    let mut now = stream("cargo build");
    let mut before = StreamBefore::of("cargo build");

    let told = now.push(&line.repeat(just_over));

    assert_eq!(told, before.push(&line.repeat(just_over)));
    assert!(
        told.contains(&format!("[+{just_over} lines: build progress]")),
        "{told}"
    );
}

#[test]
fn keeps_the_colours_and_the_redrawn_lines_when_not_cleaning() {
    let mut stream = LineStream::of(
        &CommandLine::read("python3 train.py"),
        Toggle::Off,
        Toggle::On,
    );

    let told = stream.push("\u{1b}[32m10%\u{1b}[0m\r20%\n");

    assert_eq!(told, "\u{1b}[32m10%\u{1b}[0m\r20%\n");
}

#[test]
fn counts_no_progress_when_not_counting() {
    let mut stream = LineStream::of(&CommandLine::read("cargo build"), Toggle::On, Toggle::Off);
    let build = "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n   Compiling d v1.0.0\n";

    let mut told = stream.push(build);
    told.push_str(&stream.rest());

    assert_eq!(told, build);
}
