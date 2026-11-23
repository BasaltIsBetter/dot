use crate::report::rules::{Lines, PASSED_TESTS, Rule, contains_any};

const PASSED_HOOKS: &str = "lines: hooks that passed";
const PASSED_CHECKS: &str = "lines: checks that passed";

const CARGO_TEST: &str = "test ";
const PASSED_AFTER_DOTS: &str = " ... ok";
const PYTEST_FILE: &str = ".py ";
const PYTEST_SHARE: &str = "%]";
const PYTEST_MARKS: [char; 3] = ['.', 's', ' '];
const PYTEST_TEST: &str = "::";
const PASSED_WORD: &str = " PASSED";
const CHECK_MARKS: [char; 3] = ['✓', '√', '✔'];
const INTRODUCES_WHAT_FOLLOWS: char = ':';
const GO_STEPS: [&str; 5] = ["--- PASS:", "=== CONT", "=== NAME", "=== PAUSE", "=== RUN"];
const MAVEN_CLASS: &str = "[INFO] Running ";
const MAVEN_COUNT: &str = "[INFO] Tests run:";
const MAVEN_CLEAN: &str = "Failures: 0, Errors: 0";
const MAVEN_OF_A_CLASS: &str = " in ";
const XCTEST_CASE: &str = "Test Case '";
const XCTEST_SUITE: &str = "Test Suite '";
const XCTEST_CASE_STEPS: [&str; 2] = ["' started.", "' passed ("];
const XCTEST_SUITE_STEPS: [&str; 2] = ["' started at ", "' passed at "];
const CTEST_START: &str = "Start ";
const CTEST_TEST: &str = " Test ";
const CTEST_PASSED: &str = "   Passed ";
const DENO_PASSED: &str = " ... ok (";
const SCALATEST: &str = "[info] - ";
const SCALATEST_FAILED: &str = "*** FAILED ***";
const HOOK_DOTS: &str = "....";
const HOOK_PASSED: [&str; 2] = ["Passed", "Skipped"];

const TEST_RUNNERS: [&str; 10] = [
    "--test",
    "ava",
    "bats",
    "jest",
    "mocha",
    "pest",
    "phpunit",
    "playwright",
    "swift",
    "vitest",
];
const LAUNCHERS: [&str; 8] = ["bun", "deno", "npm", "npx", "nx", "pnpm", "turbo", "yarn"];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["cargo"],
        what: PASSED_TESTS,
        lines: Lines::Told(cargo_passed),
    },
    Rule {
        tools: &["cargo", "nextest"],
        what: PASSED_TESTS,
        lines: Lines::Indented(&["PASS ["]),
    },
    Rule {
        tools: &["pytest", "py.test", "tox", "nox"],
        what: PASSED_TESTS,
        lines: Lines::Told(pytest_passed),
    },
    Rule {
        tools: &["unittest", "manage.py"],
        what: PASSED_TESTS,
        lines: Lines::Ending(&[PASSED_AFTER_DOTS]),
    },
    Rule {
        tools: &TEST_RUNNERS,
        what: PASSED_TESTS,
        lines: Lines::Told(check_marked),
    },
    Rule {
        tools: &LAUNCHERS,
        what: PASSED_CHECKS,
        lines: Lines::Told(check_marked),
    },
    Rule {
        tools: &["jest", "npm", "npx", "nx", "pnpm", "turbo", "yarn"],
        what: PASSED_TESTS,
        lines: Lines::Starting(&["PASS "]),
    },
    Rule {
        tools: &["go", "gotestsum"],
        what: PASSED_TESTS,
        lines: Lines::Indented(&GO_STEPS),
    },
    Rule {
        tools: &["dotnet"],
        what: PASSED_TESTS,
        lines: Lines::Indented(&["Passed "]),
    },
    Rule {
        tools: &["gradle", "gradlew"],
        what: PASSED_TESTS,
        lines: Lines::Ending(&[PASSED_WORD]),
    },
    Rule {
        tools: &["mvn", "mvnw"],
        what: PASSED_TESTS,
        lines: Lines::Told(maven_class_passed),
    },
    Rule {
        tools: &["swift", "xcodebuild"],
        what: PASSED_TESTS,
        lines: Lines::Told(xctest_step),
    },
    Rule {
        tools: &["ctest"],
        what: PASSED_TESTS,
        lines: Lines::Told(ctest_passed),
    },
    Rule {
        tools: &["deno"],
        what: PASSED_TESTS,
        lines: Lines::Told(deno_passed),
    },
    Rule {
        tools: &["tap", "prove"],
        what: PASSED_TESTS,
        lines: Lines::Starting(&["ok "]),
    },
    Rule {
        tools: &["sbt"],
        what: PASSED_TESTS,
        lines: Lines::Told(scalatest_passed),
    },
    Rule {
        tools: &["pre-commit"],
        what: PASSED_HOOKS,
        lines: Lines::Told(hook_passed),
    },
];

fn cargo_passed(lines: &[&str], at: usize) -> bool {
    lines[at].starts_with(CARGO_TEST) && lines[at].ends_with(PASSED_AFTER_DOTS)
}

fn pytest_passed(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_end();
    file_passed(line) || (line.contains(PYTEST_TEST) && test_passed(line))
}

fn file_passed(line: &str) -> bool {
    let Some((_, results)) = line.split_once(PYTEST_FILE) else {
        return false;
    };
    let Some((marks, share)) = results.rsplit_once('[') else {
        return false;
    };
    share.ends_with(PYTEST_SHARE)
        && !marks.trim().is_empty()
        && marks.chars().all(|mark| PYTEST_MARKS.contains(&mark))
}

fn test_passed(line: &str) -> bool {
    let result = line.rsplit_once('[').map_or(line, |(result, share)| {
        if share.ends_with(PYTEST_SHARE) {
            result
        } else {
            line
        }
    });
    result.trim_end().ends_with(PASSED_WORD)
}

fn check_marked(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim();
    line.starts_with(CHECK_MARKS) && !line.ends_with(INTRODUCES_WHAT_FOLLOWS)
}

fn maven_class_passed(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let counted_clean = line.starts_with(MAVEN_COUNT)
        && line.contains(MAVEN_CLEAN)
        && line.contains(MAVEN_OF_A_CLASS);
    line.starts_with(MAVEN_CLASS) || counted_clean
}

fn xctest_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    (line.starts_with(XCTEST_CASE) && contains_any(line, &XCTEST_CASE_STEPS))
        || (line.starts_with(XCTEST_SUITE) && contains_any(line, &XCTEST_SUITE_STEPS))
}

fn ctest_passed(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_start();
    line.starts_with(CTEST_START) || (line.contains(CTEST_TEST) && line.contains(CTEST_PASSED))
}

fn deno_passed(lines: &[&str], at: usize) -> bool {
    lines[at].contains(DENO_PASSED)
}

fn scalatest_passed(lines: &[&str], at: usize) -> bool {
    lines[at].starts_with(SCALATEST) && !lines[at].contains(SCALATEST_FAILED)
}

fn hook_passed(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_end();
    line.contains(HOOK_DOTS) && HOOK_PASSED.iter().any(|result| line.ends_with(result))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-compact/report/rules/passed.rs"]
mod tests;
