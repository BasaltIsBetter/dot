use super::*;

const STYLISH: &str = "\n/Users/me/project/src/app.js\n   1:10  error  'x' is defined but never used  no-unused-vars\n   2:5   warning  Unexpected console statement  no-console\n\n/Users/me/project/src/lib/util.js\n  14:1  error  Missing semicolon  semi\n\n✖ 3 problems (2 errors, 1 warning)\n";

#[test]
fn writes_once_the_folder_the_linted_files_share() {
    assert_eq!(
        name_folder(STYLISH).as_deref(),
        Some(
            "[paths under /Users/me/project/src/]\n\napp.js\n   1:10  error  'x' is defined but never used  no-unused-vars\n   2:5   warning  Unexpected console statement  no-console\n\nlib/util.js\n  14:1  error  Missing semicolon  semi\n\n✖ 3 problems (2 errors, 1 warning)\n"
        )
    );
}

#[test]
fn keeps_the_end_of_the_output_as_it_was() {
    let unfinished = STYLISH.trim_end();

    assert!(
        name_folder(unfinished)
            .is_some_and(|named| named.ends_with("✖ 3 problems (2 errors, 1 warning)"))
    );
}

#[test]
fn says_nothing_when_no_folder_is_worth_naming() {
    let one_file = "/Users/me/project/src/app.js\n   1:10  error  Missing semicolon  semi\n";
    let relative = "config/app.yml\n  3:1  warning  missing document start  (document-start)\nconfig/db.yml\n  5:2  error  too many spaces  (braces)\n";
    let short = "/tmp/a.js\n  1:1  error  x  y\n/tmp/b.js\n  2:1  error  x  y\n";

    assert_eq!(name_folder(one_file), None);
    assert_eq!(name_folder(relative), None);
    assert_eq!(name_folder(short), None);
    assert_eq!(name_folder(""), None);
}

#[test]
fn a_path_only_heads_findings_when_one_follows_it() {
    let lines = [
        "/Users/me/project/src/app.js",
        "   1:10  error  Missing semicolon  semi",
        "/Users/me/project/src/lib.js",
        "plain text",
        "/Users/me/my project/src/lib.js",
        "   1:10  error  Missing semicolon  semi",
        "/Users/me/project/src/last.js",
    ];

    let heads: Vec<bool> = (0..lines.len())
        .map(|at| heads_findings(&lines, at))
        .collect();

    assert_eq!(heads, [true, false, false, false, false, false, false]);
}

#[test]
fn a_finding_is_indented_and_starts_with_its_position() {
    assert!(is_finding("   1:10  error  Missing semicolon  semi"));
    assert!(is_finding("\t14:1 warning"));
    for line in [
        "1:10  error  not indented",
        "  12:x  error",
        "  12  error",
        "  :5  error",
        "  12:  error",
        "  error 12:5",
        "   ",
        "",
    ] {
        assert!(!is_finding(line), "{line}");
    }
}
