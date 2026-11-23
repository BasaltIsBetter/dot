use super::*;

fn matched(line: &str) -> Option<Hit<'_>> {
    numbered(line).or_else(|| positioned(line))
}

fn read(line: &str) -> Option<(&str, &str)> {
    matched(line).map(|hit| (hit.path, hit.row))
}

#[test]
fn reads_the_line_of_a_search() {
    assert_eq!(
        read("src/a.rs:12:    let x = 1;"),
        Some(("src/a.rs", "12:    let x = 1;"))
    );
    assert_eq!(
        read("src/a.py:3:8: F401 `os` imported but unused"),
        Some(("src/a.py", "3:8: F401 `os` imported but unused"))
    );
}

#[test]
fn reads_a_finding_named_by_the_code_of_its_rule() {
    assert_eq!(
        read("Dockerfile:3 DL3008 warning: Pin versions in apt-get install"),
        Some((
            "Dockerfile",
            "3 DL3008 warning: Pin versions in apt-get install"
        ))
    );
    assert_eq!(
        read("docs/README.md:3 MD012/no-multiple-blanks Multiple consecutive blank lines"),
        Some((
            "docs/README.md",
            "3 MD012/no-multiple-blanks Multiple consecutive blank lines"
        ))
    );
    assert_eq!(read("build.sh:7 SC2086"), Some(("build.sh", "7 SC2086")));
    assert_eq!(
        read("app.py:1 ABCD123 four letters"),
        Some(("app.py", "1 ABCD123 four letters"))
    );
}

#[test]
fn reads_a_position_between_parentheses() {
    assert_eq!(
        read("src/app.ts(12,5): error TS2322: Type 'string' is not assignable to type 'number'."),
        Some((
            "src/app.ts",
            "(12,5): error TS2322: Type 'string' is not assignable to type 'number'."
        ))
    );
    assert_eq!(
        read("/work/app/Program.cs(8): warning CS0168: unused"),
        Some(("/work/app/Program.cs", "(8): warning CS0168: unused"))
    );
    assert_eq!(
        read("Program.cs(8,5,8,20): error CS1002: ; expected"),
        Some(("Program.cs", "(8,5,8,20): error CS1002: ; expected"))
    );
}

#[test]
fn does_not_take_these_for_findings() {
    for line in [
        "localhost:8080 GET /health",
        "host:22 SSH2 ready",
        "build.sh:7 Ab123 lower case in the code",
        "build.sh:7 ABCDE123 five letters",
        "build.sh:7 AB12 two digits",
        "build.sh:7 A123 one letter",
        "build.sh:7 AB123x goes on",
        "build.sh:x SC2086",
        "two words:7 SC2086",
        "12:7 SC2086",
        ":7 SC2086",
        "src/app.ts(x,5): error",
        "src/app.ts(12,): error",
        "src/app.ts(1,2,3,4,5): error",
        "src/app.ts(12,5) error",
        "src/app.ts(12,5):error",
        "(12,5): no path",
        "two words(12,5): error",
        "12(3,4): only digits",
        "no position at all",
        "time: 12:30",
        "12:34:56 started",
        "note that:1:this has spaces",
        ":1:no path",
        "src/a.rs:x:not a number",
        "src/a.rs:12",
    ] {
        assert_eq!(read(line), None, "{line}");
    }
}

#[test]
fn reads_a_line_of_context_only_for_a_file_that_matched() {
    let files = BTreeSet::from(["src/a-b.rs"]);
    let read = |line| context(line, &files).map(|hit| (hit.path, hit.row));

    assert_eq!(
        read("src/a-b.rs-11-    let before = 0;"),
        Some(("src/a-b.rs", "11-    let before = 0;"))
    );
    assert_eq!(read("src/other.rs-11-    let before = 0;"), None);
    assert_eq!(read("src/a-b.rs-x-    not a number"), None);
    assert_eq!(read("src/a-b.rs-12"), None);
    assert_eq!(read("no dash at all"), None);
}

#[test]
fn a_number_is_made_of_digits_only() {
    assert!(is_number("120"));
    assert!(!is_number(""));
    assert!(!is_number("12a"));
    assert!(!is_number(" 12"));
}

#[test]
fn only_a_line_of_a_search_may_be_cut() {
    let whole = |line| matched(line).map(|hit| hit.whole);

    assert_eq!(whole("src/a.rs:12:    let x = 1;"), Some(false));
    assert_eq!(
        whole("docs/a.md:10:1 MD022/blanks-around-headings"),
        Some(false)
    );
    assert_eq!(
        whole("Dockerfile:3 DL3008 warning: Pin versions"),
        Some(true)
    );
    assert_eq!(whole("src/app.ts(12,5): error TS2322: Type"), Some(true));
}

#[test]
fn does_not_take_the_time_of_a_log_line_for_a_search_result() {
    for line in [
        "[21:34:10 INFO]: Loading chunk region r.10.0.mca",
        "[21:34:10] Finished 'build' after 10 ms",
        "2026-11-19T21:34:10.120Z INFO request handled",
        "app-21:34:10 started",
    ] {
        assert_eq!(read(line), None, "{line}");
    }
    for line in [
        "src/v2:34:10 apples",
        "notes21:5:10 apples",
        "notes21:34:5 apples",
        "notes21:34:567",
        "notes121:34:56 apples",
        "notes21:34:apples",
    ] {
        assert!(read(line).is_some(), "{line}");
    }
}
