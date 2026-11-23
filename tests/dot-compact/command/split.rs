use proptest::prelude::*;

use super::*;

fn programs(line: &str) -> Vec<(&str, bool)> {
    invocations(line)
        .into_iter()
        .map(|invocation| (invocation.program, invocation.piped))
        .collect()
}

#[test]
fn reads_a_command_and_its_arguments() {
    assert_eq!(
        invocations("grep -rn foo src"),
        [Invocation {
            program: "grep",
            arguments: vec!["-rn", "foo", "src"],
            piped: false,
        }]
    );
}

#[test]
fn tells_what_reads_a_pipe_from_what_starts_a_command() {
    assert_eq!(
        programs("find . -name '*.rs' | head -5"),
        [("find", false), ("head", true)]
    );
    assert_eq!(
        programs("a && b; c || d\ne"),
        [
            ("a", false),
            ("b", false),
            ("c", false),
            ("d", false),
            ("e", false)
        ]
    );
    assert_eq!(
        programs("ls | sort | uniq"),
        [("ls", false), ("sort", true), ("uniq", true)]
    );
}

#[test]
fn skips_assignments_and_words_that_only_introduce_a_command() {
    assert_eq!(
        programs("FOO=1 BAR=2 sudo env cargo test"),
        [("cargo", false)]
    );
    assert_eq!(programs("time ./gradlew build"), [("gradlew", false)]);
    assert_eq!(programs("/usr/bin/grep x y"), [("grep", false)]);
}

#[test]
fn an_assignment_has_a_name_made_of_letters_digits_and_underscores() {
    assert_eq!(programs("MY_VAR_2=1 cargo test"), [("cargo", false)]);
    assert_eq!(programs("=x"), [("=x", false)]);
    assert_eq!(programs("a-b=c d"), [("a-b=c", false)]);
    assert_eq!(programs("./run=fast now"), [("run=fast", false)]);
}

#[test]
fn keeps_quoted_operators_inside_their_word() {
    let found = invocations(r#"grep "a | b; c" 'd && e' file\ name | wc -l"#);

    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0].arguments,
        [r#""a | b; c""#, "'d && e'", r"file\ name"]
    );
    assert_eq!((found[1].program, found[1].piped), ("wc", true));
}

#[test]
fn does_not_take_a_redirection_for_a_background_job() {
    let found = invocations("cargo test 2>&1 | tail -5");

    assert_eq!(found[0].arguments, ["test", "2>&1"]);
    assert_eq!(
        programs("cargo test &>log | tail"),
        [("cargo", false), ("tail", true)]
    );
    assert_eq!(programs("sleep 5 & ls"), [("sleep", false), ("ls", false)]);
}

#[test]
fn a_pipe_that_carries_the_errors_too_is_still_a_pipe() {
    assert_eq!(
        programs("cargo test |& tail -5"),
        [("cargo", false), ("tail", true)]
    );
    assert_eq!(
        programs("a || b & c"),
        [("a", false), ("b", false), ("c", false)]
    );
}

#[test]
fn finds_the_commands_inside_loops_substitutions_and_find() {
    assert_eq!(
        programs(r#"for f in a b; do cat "$f"; done"#),
        [("for", false), ("cat", false), ("done", false)]
    );
    assert_eq!(programs("echo $(cat x)"), [("echo", false), ("cat", false)]);
    assert_eq!(
        programs(r"find . -exec cat {} \;"),
        [("find", false), ("cat", false)]
    );
}

#[test]
fn a_command_run_by_xargs_reads_files_not_the_pipe() {
    assert_eq!(
        programs("find . | xargs cat"),
        [("find", false), ("cat", false)]
    );
    assert_eq!(
        programs("find . | xargs -n 1 grep -n x"),
        [("find", false), ("grep", false)]
    );
}

#[test]
fn an_empty_line_runs_nothing() {
    assert_eq!(invocations(""), []);
    assert_eq!(invocations("  ;; | "), []);
}

proptest! {
    #[test]
    fn any_line_is_read_without_panicking(line in any::<String>()) {
        for invocation in invocations(&line) {
            prop_assert!(!invocation.program.is_empty());
            prop_assert!(line.contains(invocation.program));
        }
    }

    #[test]
    fn a_word_alone_is_the_program(word in "[a-z][a-z0-9_]{0,12}") {
        prop_assume!(!PREFIXES.contains(&word.as_str()));
        prop_assert_eq!(programs(&word), [(word.as_str(), false)]);
    }
}
