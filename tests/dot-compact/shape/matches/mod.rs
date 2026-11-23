use proptest::prelude::*;

use super::hit::{CONTEXT, FIELD};
use super::*;

fn matched(line: &str) -> Option<Hit<'_>> {
    numbered(line).or_else(|| positioned(line))
}

fn ungrouped(grouped: &str) -> Vec<String> {
    let mut common = "";
    let mut heading = "";
    let mut lines = Vec::new();
    for line in grouped.lines() {
        if let Some(folder) = line
            .strip_prefix("[paths under ")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            common = folder;
        } else if line.is_empty() {
        } else if matched(line).is_some() {
            lines.push(format!("{common}{line}"));
        } else if line.starts_with(|first: char| first.is_ascii_digit()) {
            let separator = line
                .find(|c: char| !c.is_ascii_digit())
                .map_or(FIELD, |at| line[at..].chars().next().unwrap());
            lines.push(format!("{common}{heading}{separator}{line}"));
        } else {
            heading = line;
        }
    }
    lines
}

#[test]
fn writes_the_path_of_several_matches_once() {
    let found = "src/a.rs:1:one\nsrc/a.rs:22:two\nsrc/a.rs:23:three\nsrc/b.rs:4:four\n";

    assert_eq!(
        group(found).as_deref(),
        Some("src/a.rs\n1:one\n22:two\n23:three\nsrc/b.rs:4:four\n")
    );
}

#[test]
fn separates_two_groups_with_a_blank_line() {
    let found = "src/a.rs:1:one\nsrc/a.rs:2:two\nsrc/b.rs:3:three\nsrc/b.rs:4:four\n";

    assert_eq!(
        group(found).as_deref(),
        Some("src/a.rs\n1:one\n2:two\n\nsrc/b.rs\n3:three\n4:four\n")
    );
}

#[test]
fn keeps_context_lines_and_their_breaks_with_their_file() {
    let found = "src/a-b.rs-1-before\nsrc/a-b.rs:2:hit\n--\nsrc/a-b.rs:9:hit\nsrc/a-b.rs-10-after\nelsewhere\n";

    assert_eq!(
        group(found).as_deref(),
        Some("src/a-b.rs\n1-before\n2:hit\n--\n9:hit\n10-after\nelsewhere\n")
    );
}

#[test]
fn leaves_alone_what_has_too_few_matches_or_nothing_to_gain() {
    assert_eq!(
        group("src/a.rs:1:one\nsrc/a.rs:2:two\nsrc/a.rs:3:three\n"),
        None
    );
    assert_eq!(group("a.rs:1:x\nb.rs:2:x\nc.rs:3:x\nd.rs:4:x\n"), None);
    assert_eq!(
        group("plain text\nwith: colons\n12:34:56 a time\nkey: 1:2\n"),
        None
    );
}

#[test]
fn names_the_folder_absolute_paths_share() {
    let found = "/Users/me/project/src/a.rs:1:one\n/Users/me/project/src/a.rs:2:two\n/Users/me/project/lib/b.rs:3:three\n/Users/me/project/lib/c.rs:4:four\n";

    assert_eq!(
        group(found).as_deref(),
        Some(
            "[paths under /Users/me/project/]\n\nsrc/a.rs\n1:one\n2:two\nlib/b.rs:3:three\nlib/c.rs:4:four\n"
        )
    );
}

#[test]
fn cuts_a_text_longer_than_400_bytes_and_counts_what_is_cut() {
    let long = "é".repeat(300);
    let found = format!("a.rs:1:{long}\na.rs:2:short\na.rs:3:short\na.rs:4:short\n");

    let grouped = group(&found).unwrap();

    let first_row = grouped.lines().nth(1).unwrap();
    assert!(first_row.starts_with("1:éé"));
    assert!(
        first_row.ends_with(&left_out(101, CUT_CHARACTERS)),
        "{first_row}"
    );
    assert_eq!(
        first_row.len(),
        LONGEST_TEXT + left_out(101, CUT_CHARACTERS).len()
    );
}

#[test]
fn a_text_of_exactly_400_bytes_is_whole() {
    let row = format!("1:{}", "x".repeat(LONGEST_TEXT - 2));

    assert_eq!(cut_at(&row), LONGEST_TEXT);
    assert_eq!(cut_at(&format!("{row}y")), LONGEST_TEXT);
}

#[test]
fn keeps_a_lone_context_line_as_it_was_written() {
    let found = "a.rs:1:x\nb.rs:2:x\nb.rs:3:x\na.rs-4-after\nb.rs:5:x\n";

    assert_eq!(
        group(&format!("{found}b.rs:6:x\nb.rs:7:x\nb.rs:8:x\nb.rs:9:x\n")).as_deref(),
        Some("a.rs:1:x\n\nb.rs\n2:x\n3:x\na.rs-4-after\n\nb.rs\n5:x\n6:x\n7:x\n8:x\n9:x\n")
    );
}

#[test]
fn ends_like_the_original() {
    let found = "a.rs:1:x\na.rs:2:x\na.rs:3:x\na.rs:4:x";

    assert_eq!(group(found).as_deref(), Some("a.rs\n1:x\n2:x\n3:x\n4:x"));
}

proptest! {
    #[test]
    fn every_match_is_told_in_the_same_order(
        hits in prop::collection::vec((0_usize..4, 1_u32..500, "[a-z(){}; ]{1,30}", any::<bool>()), 4..40),
        absolute in any::<bool>(),
    ) {
        let root = if absolute { "/Users/me/a-long-project-folder/" } else { "" };
        let files = ["src/a.rs", "src/deep/b.rs", "c-d.rs", "lib/e_f.rs"];
        let mut lines: Vec<String> = Vec::new();
        let mut previous = None;
        for (file, number, text, is_context) in &hits {
            let follows_its_file = previous == Some(file);
            let separator = if *is_context && follows_its_file { CONTEXT } else { FIELD };
            lines.push(format!("{root}{}{separator}{number}{separator}{}", files[*file], text.trim_end()));
            previous = Some(file);
        }
        let found = lines.join("\n");

        if let Some(grouped) = group(&found) {
            prop_assert!(grouped.len() < found.len());
            prop_assert_eq!(ungrouped(&grouped), lines);
        }
    }

    #[test]
    fn any_text_is_grouped_without_panicking(text in any::<String>()) {
        if let Some(grouped) = group(&text) {
            prop_assert!(grouped.len() < text.len());
        }
    }
}

#[test]
fn groups_what_a_compiler_says_under_each_file() {
    let errors = "src/app.ts(12,5): error TS2322: Type 'string' is not assignable to type 'number'.\nsrc/app.ts(30,1): error TS2304: Cannot find name 'x'.\nsrc/lib/util.ts(4,9): error TS7006: Parameter 'a' implicitly has an 'any' type.\nsrc/lib/util.ts(9,2): error TS2345: Argument of type 'null' is not assignable.\n\nFound 4 errors in 2 files.\n";

    assert_eq!(
        group(errors).as_deref(),
        Some(
            "src/app.ts\n(12,5): error TS2322: Type 'string' is not assignable to type 'number'.\n(30,1): error TS2304: Cannot find name 'x'.\n\nsrc/lib/util.ts\n(4,9): error TS7006: Parameter 'a' implicitly has an 'any' type.\n(9,2): error TS2345: Argument of type 'null' is not assignable.\n\nFound 4 errors in 2 files.\n"
        )
    );
}

#[test]
fn groups_the_findings_of_a_linter_under_each_file() {
    let findings = "docs/guide/install.md:3 MD012/no-multiple-blanks Multiple consecutive blank lines\ndocs/guide/install.md:10:1 MD022/blanks-around-headings Headings should be surrounded by blank lines\ndocs/guide/install.md:15:80 MD013/line-length Line length\nDockerfile:3 DL3008 warning: Pin versions in apt-get install\n";

    assert_eq!(
        group(findings).as_deref(),
        Some(
            "docs/guide/install.md\n3 MD012/no-multiple-blanks Multiple consecutive blank lines\n10:1 MD022/blanks-around-headings Headings should be surrounded by blank lines\n15:80 MD013/line-length Line length\nDockerfile:3 DL3008 warning: Pin versions in apt-get install\n"
        )
    );
}

#[test]
fn keeps_what_is_shown_under_a_finding_with_its_file() {
    let findings = "internal/server/handler.go:12:2: ineffectual assignment to err (ineffassign)\n\terr = run()\n\t^\ninternal/server/handler.go:30:5: unused variable (unused)\n\tx := 1\n\t^\ninternal/server/handler.go:41:1: missing return\nother.go:1:1: lone\n  shown under the lone one\nplain text\n";

    assert_eq!(
        group(findings).as_deref(),
        Some(
            "internal/server/handler.go\n12:2: ineffectual assignment to err (ineffassign)\n\terr = run()\n\t^\n30:5: unused variable (unused)\n\tx := 1\n\t^\n41:1: missing return\nother.go:1:1: lone\n  shown under the lone one\nplain text\n"
        )
    );
}

#[test]
fn never_cuts_what_a_compiler_says() {
    let long = "x".repeat(500);
    let errors = format!(
        "a.ts(1,1): error TS1: {long}\na.ts(2,1): error TS2: short\na.ts(3,1): error TS3: short\na.ts(4,1): error TS4: short\nb.ts(1,1): error TS1: {long}\n"
    );

    let grouped = group(&errors).unwrap();

    assert!(grouped.starts_with(&format!("a.ts\n(1,1): error TS1: {long}\n(2,1): ")));
    assert!(grouped.ends_with(&format!("b.ts(1,1): error TS1: {long}\n")));
    assert!(!grouped.contains(CUT_CHARACTERS));
}

#[test]
fn counts_the_indented_lines_that_follow() {
    assert_eq!(shown_under(&["\ta", "  b", "c", " d"]), 2);
    assert_eq!(shown_under(&["a", " b"]), 0);
    assert_eq!(shown_under(&[]), 0);
}

#[test]
fn a_break_only_joins_two_matches_of_the_same_file() {
    let found = "a.rs:1:one\n--\nb.rs:2:two\nb.rs:3:three\nc.rs:4:four\nc.rs:5:five\n";

    assert_eq!(
        group(found).as_deref(),
        Some("a.rs:1:one\n--\n\nb.rs\n2:two\n3:three\n\nc.rs\n4:four\n5:five\n")
    );
}

#[test]
fn a_run_ends_with_the_last_match_of_its_file_after_a_break() {
    let found = "a.rs:1:x\na.rs:2:x\na.rs:3:x\n--\na.rs:9:x\nb.rs:1:y\nb.rs:2:y\n";

    assert_eq!(
        group(found).as_deref(),
        Some("a.rs\n1:x\n2:x\n3:x\n--\n9:x\n\nb.rs\n1:y\n2:y\n")
    );
}
