use super::*;

fn compact(command: &CommandLine<'_>, text: &str) -> Option<String> {
    let rewritten = rewrite(command, text);
    let current = rewritten.as_deref().unwrap_or(text);
    count(command, current).or(rewritten)
}

#[test]
fn passes_the_output_through_every_report_that_knows_the_command() {
    let output = "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n    Finished `dev` profile in 2.1s\ntest x::one ... ok\ntest x::two ... ok\ntest x::three ... ok\ntest result: ok. 3 passed\n";
    let command = CommandLine::read("cargo test");

    assert_eq!(
        compact(&command, output).as_deref(),
        Some(
            "[+3 lines: build progress]\n    Finished `dev` profile in 2.1s\n[+3 lines: tests that passed]\ntest result: ok. 3 passed\n"
        )
    );
}

#[test]
fn says_nothing_changed_for_a_command_no_report_knows() {
    let output = "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n";

    assert_eq!(
        compact(&CommandLine::read("python3 build.py"), output),
        None
    );
}

#[test]
fn rewrites_without_counting_and_counts_without_rewriting() {
    let output = "On branch main\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n\tmodified:   a.rs\n";
    let status = CommandLine::read("git status");

    assert!(rewrite(&status, output).is_some());
    assert_eq!(count(&status, output), None);

    let build = "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n";
    let cargo = CommandLine::read("cargo build");
    assert_eq!(rewrite(&cargo, build), None);
    assert!(count(&cargo, build).is_some());
}
