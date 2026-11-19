use std::path::Path;

use super::*;

#[test]
fn a_single_star_stays_within_one_folder() {
    let matcher = glob("src/*.rs").unwrap();

    assert!(matcher.is_match("src/main.rs"));
    assert!(!matcher.is_match("src/deep/main.rs"));
    assert!(glob("src/**/*.rs").unwrap().is_match("src/deep/main.rs"));
}

#[test]
fn explains_a_broken_pattern() {
    let error = glob("src/[").unwrap_err();

    assert!(error.starts_with("The glob pattern is not valid"));
}

#[test]
fn a_pattern_without_a_slash_matches_file_names_anywhere() {
    let by_name = glob("*.rs").unwrap();
    let by_path = glob("src/*.rs").unwrap();

    assert!(matches_name_or_path(
        &by_name,
        "*.rs",
        Path::new("deep/down/a.rs")
    ));
    assert!(!matches_name_or_path(
        &by_name,
        "*.rs",
        Path::new("deep/down/a.txt")
    ));
    assert!(matches_name_or_path(
        &by_path,
        "src/*.rs",
        Path::new("src/a.rs")
    ));
    assert!(!matches_name_or_path(
        &by_path,
        "src/*.rs",
        Path::new("lib/src/a.rs")
    ));
}

#[test]
fn counts_into_a_number_claude_reads() {
    assert_eq!(count(0), 0);
    assert_eq!(count(7), 7);
}
