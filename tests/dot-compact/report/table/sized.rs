use super::*;

const LINES: &str = "     136 crates/dot-compact/src/report/listing.rs\n      25 crates/dot-compact/src/report/mod.rs\n      59 crates/dot-compact/src/report/progress.rs\n      63 crates/dot-compact/src/report/status.rs\n      76 crates/dot-compact/src/report/stream.rs\n     359 total\n";

fn sized(command: &str, text: &str) -> Option<String> {
    compact(&CommandLine::read(command), text)
}

#[test]
fn writes_once_the_folder_the_counted_files_share() {
    assert_eq!(
        sized("wc -l crates/dot-compact/src/report/*.rs", LINES).as_deref(),
        Some(
            "[paths under crates/dot-compact/src/report/]\n136 listing.rs\n25 mod.rs\n59 progress.rs\n63 status.rs\n76 stream.rs\n359 total\n"
        )
    );
}

#[test]
fn keeps_every_count_of_a_line() {
    let counts = "      30      96     978 /Users/me/project/src/a.py\n      12      40     310 /Users/me/project/src/b c.py\n       5      11      90 /Users/me/project/README.md\n      47     147    1378 total";

    assert_eq!(
        sized(
            "wc /Users/me/project/src/*.py /Users/me/project/README.md",
            counts
        )
        .as_deref(),
        Some(
            "[paths under /Users/me/project/]\n30 96 978 src/a.py\n12 40 310 src/b c.py\n5 11 90 README.md\n47 147 1378 total"
        )
    );
}

#[test]
fn reads_the_sizes_du_gives() {
    let sizes = " 12K\tcrates/dot-compact/src/command\n4.0K\tcrates/dot-compact/src/compactor.rs\n1,5M\tcrates/dot-compact/src/2024 notes\n  0B\tcrates/dot-compact/src/lib.rs\n";

    assert_eq!(
        sized("du -sh crates/dot-compact/src/*", sizes).as_deref(),
        Some(
            "[paths under crates/dot-compact/src/]\n12K command\n4.0K compactor.rs\n1,5M 2024 notes\n0B lib.rs\n"
        )
    );
}

#[test]
fn needs_four_lines_and_a_folder_worth_naming() {
    let three = LINES.lines().take(3).collect::<Vec<&str>>().join("\n");
    let four = LINES.lines().take(4).collect::<Vec<&str>>().join("\n");
    let apart = "  1 crates/dot-compact/src/lib.rs\n  2 docs/guide/intro.md\n  3 tests/dot-compact/note.rs\n  4 benches/compaction.rs\n";
    let short = "  1 src/a.rs\n  2 src/b.rs\n  3 src/c.rs\n  4 src/d.rs\n";

    assert_eq!(sized("wc -l", &three), None);
    assert!(sized("wc -l", &four).is_some());
    assert_eq!(sized("wc -l", apart), None);
    assert_eq!(sized("wc -l", short), None);
    assert_eq!(sized("wc -l", ""), None);
}

#[test]
fn keeps_an_output_with_a_line_it_cannot_read() {
    let with_an_error = format!("wc: nope.txt: open: No such file or directory\n{LINES}");
    let with_a_gap = LINES.replacen('\n', "\n\n", 1);
    let without_a_path = LINES.replacen("crates/dot-compact/src/report/mod.rs", "", 1);

    assert_eq!(sized("wc -l", &with_an_error), None);
    assert_eq!(sized("wc -l", &with_a_gap), None);
    assert_eq!(sized("wc -l", &without_a_path), None);
}

#[test]
fn only_reads_what_du_and_wc_print() {
    assert_eq!(sized("cat counts.txt", LINES), None);
    assert!(sized("find . -name '*.rs' | xargs wc -l", LINES).is_some());
}

#[test]
fn the_total_says_how_many_counts_a_line_has() {
    assert_eq!(count_columns(&["  3 a", "  9 total"]), 1);
    assert_eq!(count_columns(&[" 1 2 3 a", " 1 2 3 total"]), 3);
    assert_eq!(count_columns(&["4.0K\ta", "12K\tb"]), 1);
    assert_eq!(count_columns(&["total", "a total"]), 1);
    assert_eq!(count_columns(&["  3 2024 notes", "  9 total"]), 1);
    assert_eq!(count_columns(&["4.0K\tx", "a b total"]), 1);
    assert_eq!(count_columns(&["4.0K\tsome folder/x"]), 1);
    assert_eq!(count_columns(&["total"]), 1);
    assert_eq!(count_columns(&[]), 1);
}

#[test]
fn a_count_is_a_number_and_a_short_unit() {
    for count in ["136", "4.0K", "1,5M", "12Ki", "0B"] {
        assert!(is_count(count), "{count}");
    }
    for word in ["12KiB", "K", "", "1a2", "total", "src/12", ".5K"] {
        assert!(!is_count(word), "{word}");
    }
}
