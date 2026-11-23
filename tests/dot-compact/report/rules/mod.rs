use super::*;

fn never(_: &[&str], _: usize) -> bool {
    false
}

fn second_line(_: &[&str], at: usize) -> bool {
    at == 1
}

fn rule(lines: Lines) -> Rule {
    Rule {
        tools: &["tool"],
        what: BUILD_PROGRESS,
        lines,
    }
}

#[test]
fn a_rule_reads_a_line_by_its_start_its_end_or_a_function() {
    let lines = ["Compiling a", "   Compiling b", "done Compiling  "];

    assert!(rule(Lines::Starting(&["Compiling "])).drops(&lines, 0));
    assert!(!rule(Lines::Starting(&["Compiling "])).drops(&lines, 1));
    assert!(rule(Lines::Indented(&["Compiling "])).drops(&lines, 1));
    assert!(!rule(Lines::Indented(&["Compiling "])).drops(&lines, 2));
    assert!(rule(Lines::Ending(&[" Compiling"])).drops(&lines, 2));
    assert!(!rule(Lines::Ending(&[" Compiling"])).drops(&lines, 0));
    assert!(rule(Lines::Told(second_line)).drops(&lines, 1));
    assert!(!rule(Lines::Told(second_line)).drops(&lines, 0));
    assert!(!rule(Lines::Told(never)).drops(&lines, 1));
}

#[test]
fn reads_what_follows_a_counter() {
    assert_eq!(
        after_counter("[12/340] Building a.o", '[', ']'),
        Some("Building a.o")
    );
    assert_eq!(
        after_counter("[ 12%] Linking app", '[', ']'),
        Some("Linking app")
    );
    assert_eq!(
        after_counter("(3/12) Installing zlib (1.3)", '(', ')'),
        Some("Installing zlib (1.3)")
    );
    assert_eq!(after_counter("[100%]", '[', ']'), Some(""));
}

#[test]
fn does_not_take_other_brackets_for_a_counter() {
    for line in [
        "[abc] Building",
        "[] Building",
        "[/] Building",
        "[%] Building",
        "12/340] Building",
        "[12/340 Building",
        "Building [1/2]",
    ] {
        assert_eq!(after_counter(line, '[', ']'), None, "{line}");
    }
}

#[test]
fn tells_whether_a_line_starts_with_or_contains_one_of_several_texts() {
    assert!(starts_with_any("Fetching x", &["Using ", "Fetching "]));
    assert!(!starts_with_any(" Fetching x", &["Using ", "Fetching "]));
    assert!(!starts_with_any("Fetching x", &[]));
    assert!(contains_any("a: Reading...", &[": Reading..."]));
    assert!(!contains_any("a: Read", &[": Reading..."]));
}

#[test]
fn knows_these_tools_and_no_other() {
    let mut tools: Vec<&str> = RULES
        .iter()
        .flat_map(|group| group.iter())
        .flat_map(|rule| rule.tools.iter().copied())
        .collect();
    tools.sort_unstable();
    tools.dedup();

    assert_eq!(tools.len(), 108, "{tools:?}");
    for tool in [
        "gradlew",
        "mvn",
        "sbt",
        "cargo",
        "go",
        "dotnet",
        "npm",
        "pytest",
        "swift",
        "xcodebuild",
        "gcc",
        "docker",
        "terraform",
        "bundle",
        "composer",
        "curl",
        "next",
        "prettier",
        "mix",
        "glab",
    ] {
        assert!(tools.contains(&tool), "{tool}");
    }
}

#[test]
fn every_rule_says_what_it_counts() {
    for rule in RULES.iter().flat_map(|group| group.iter()) {
        assert!(rule.what.starts_with("lines: "), "{}", rule.what);
        assert_ne!(rule.tools, [""; 0]);
    }
}
