use super::*;

fn told(tool: &str, lines: &[&str]) -> Vec<Option<&'static str>> {
    (0..lines.len())
        .map(|at| {
            RULES
                .iter()
                .filter(|rule| rule.tools.contains(&tool))
                .find(|rule| rule.drops(lines, at))
                .map(|rule| rule.what)
        })
        .collect()
}

const PASSED: Option<&str> = Some(PASSED_TESTS);
const CHECKED: Option<&str> = Some(PASSED_CHECKS);

#[test]
fn rust_tests() {
    assert_eq!(
        told(
            "cargo",
            &[
                "test chat::tests::replies ... ok",
                "test chat::tests::fails ... FAILED",
                "test chat::tests::later ... ignored",
                "a test ... ok",
                "        PASS [   0.012s] dot-api chat::tests::replies",
                "        FAIL [   0.020s] dot-api chat::tests::fails",
                "test result: ok. 12 passed; 0 failed"
            ]
        ),
        [PASSED, None, None, None, PASSED, None, None]
    );
}

#[test]
fn python_tests() {
    assert_eq!(
        told(
            "pytest",
            &[
                "tests/test_a.py ...                    [ 50%]",
                "tests/test_b.py .s.                    [100%]",
                "tests/test_c.py .F.                    [100%]",
                "tests/test_d.py    [100%]",
                "see tests/test_a.py for more",
                "tests/test_e.py ..."
            ]
        ),
        [PASSED, PASSED, None, None, None, None]
    );
    assert_eq!(
        told(
            "pytest",
            &[
                "tests/test_a.py::test_one PASSED                                   [ 33%]",
                "tests/test_a.py::test_two PASSED",
                "tests/test_a.py::test_three FAILED                                 [ 66%]",
                "tests/test_a.py::test_four SKIPPED (later)                         [100%]",
                "all PASSED",
                "tests/test_a.py::test_five PASSED [not a share]"
            ]
        ),
        [PASSED, PASSED, None, None, None, None]
    );
    assert_eq!(
        told(
            "unittest",
            &[
                "test_one (test_probe.T) ... ok",
                "test_three (test_probe.T) ... skipped 'later'",
                "test_two (test_probe.T) ... FAIL",
                "Ran 3 tests in 0.000s",
                "OK (skipped=1)"
            ]
        ),
        [PASSED, None, None, None, None]
    );
}

#[test]
fn script_tests() {
    let lines = [
        " ✓ src/a.test.ts (3)",
        "✔ adds (0.387583ms)",
        "  √ subtracts",
        "﹣ later (0.039125ms) # SKIP",
        " ✗ src/b.test.ts (1)",
        "✖ fails (1ms)",
        "PASS src/a.test.ts",
        "FAIL src/b.test.ts",
        "Tests:       12 passed, 12 total",
        "a ✓ in a sentence",
    ];

    assert_eq!(
        told("jest", &lines),
        [
            PASSED, PASSED, PASSED, None, None, None, PASSED, None, None, None
        ]
    );
    for tool in ["npm", "pnpm", "yarn", "npx", "nx", "turbo"] {
        assert_eq!(
            told(tool, &lines),
            [
                CHECKED, CHECKED, CHECKED, None, None, None, PASSED, None, None, None
            ],
            "{tool}"
        );
    }
    for tool in [
        "vitest",
        "mocha",
        "playwright",
        "--test",
        "phpunit",
        "bats",
        "ava",
        "pest",
    ] {
        assert_eq!(
            told(tool, &lines),
            [
                PASSED, PASSED, PASSED, None, None, None, None, None, None, None
            ],
            "{tool}"
        );
    }
    for tool in ["bun", "deno"] {
        assert_eq!(
            told(tool, &lines),
            [
                CHECKED, CHECKED, CHECKED, None, None, None, None, None, None, None
            ],
            "{tool}"
        );
    }
}

#[test]
fn a_checked_line_that_introduces_what_follows_stays() {
    assert_eq!(
        told(
            "npx",
            &[
                "✔ Checking registry.",
                "✔ Created 1 file:",
                "  - lib/utils.ts",
                "✔ Done:  "
            ]
        ),
        [CHECKED, None, None, None]
    );
    assert_eq!(told("vitest", &[" ✓ suite:", " ✓ suite"]), [None, PASSED]);
}

#[test]
fn only_a_test_runner_named_by_the_command_makes_its_checks_tests() {
    let marks = " ✓ a\n ✓ b\n ✓ c\ndone\n";
    let counted = |command: &str| {
        crate::report::progress::compact(&crate::command::CommandLine::read(command), marks)
    };

    for command in ["npx vitest run", "pnpm exec jest", "node --test"] {
        assert_eq!(
            counted(command).as_deref(),
            Some("[+3 lines: tests that passed]\ndone\n"),
            "{command}"
        );
    }
    for command in ["npx shadcn@latest init", "npm run build", "npm test"] {
        assert_eq!(
            counted(command).as_deref(),
            Some("[+3 lines: checks that passed]\ndone\n"),
            "{command}"
        );
    }
    assert_eq!(counted("make test"), None);
}

#[test]
fn go_and_dotnet_tests() {
    assert_eq!(
        told(
            "go",
            &[
                "=== RUN   TestAdd",
                "=== PAUSE TestAdd",
                "--- PASS: TestAdd (0.00s)",
                "    --- PASS: TestAdd/small (0.00s)",
                "--- FAIL: TestSub (0.00s)",
                "--- SKIP: TestLater (0.00s)",
                "PASS",
                "ok  \texample.com/app\t0.2s"
            ]
        ),
        [PASSED, PASSED, PASSED, PASSED, None, None, None, None]
    );
    assert_eq!(
        told(
            "dotnet",
            &[
                "  Passed App.Tests.Adds [2 ms]",
                "  Failed App.Tests.Subtracts [3 ms]",
                "  Skipped App.Tests.Later",
                "Passed!  - Failed:     0, Passed:    12, Skipped:     0, Total:    12"
            ]
        ),
        [PASSED, None, None, None]
    );
}

#[test]
fn jvm_tests() {
    assert_eq!(
        told(
            "gradlew",
            &[
                "AppTest > adds() PASSED",
                "AppTest > subtracts() FAILED",
                "AppTest > later() SKIPPED",
                "PASSED tests: 12"
            ]
        ),
        [PASSED, None, None, None]
    );
    assert_eq!(
        told(
            "mvn",
            &[
                "[INFO] Running com.app.AppTest",
                "[INFO] Tests run: 3, Failures: 0, Errors: 0, Skipped: 0, Time elapsed: 0.05 s -- in com.app.AppTest",
                "[INFO] Tests run: 12, Failures: 0, Errors: 0, Skipped: 0",
                "[ERROR] Tests run: 2, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 0.01 s <<< FAILURE! -- in com.app.BadTest",
                "[INFO] Tests run: 2, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 0.01 s -- in com.app.BadTest"
            ]
        ),
        [PASSED, PASSED, None, None, None]
    );
}

#[test]
fn apple_and_c_tests() {
    assert_eq!(
        told(
            "swift",
            &[
                "Test Case '-[AppTests testAdds]' started.",
                "Test Case '-[AppTests testAdds]' passed (0.001 seconds).",
                "Test Case '-[AppTests testSubtracts]' failed (0.002 seconds).",
                "Test Suite 'AppTests' started at 2026-11-19 18:00:00.000",
                "Test Suite 'AppTests' passed at 2026-11-19 18:00:00.012",
                "Test Suite 'AppTests' failed at 2026-11-19 18:00:00.012",
                "\t Executed 12 tests, with 0 failures (0 unexpected) in 0.012 (0.014) seconds",
                "✔ Test adds() passed after 0.001 seconds."
            ]
        ),
        [PASSED, PASSED, None, PASSED, PASSED, None, None, PASSED]
    );
    assert_eq!(
        told(
            "ctest",
            &[
                "      Start  1: adds",
                " 1/12 Test  #1: adds .............................   Passed    0.01 sec",
                " 2/12 Test  #2: subtracts ........................***Failed    0.01 sec",
                "100% tests passed, 0 tests failed out of 12",
                "Total Test time (real) =   0.12 sec"
            ]
        ),
        [PASSED, PASSED, None, None, None]
    );
}

#[test]
fn deno_and_tap_tests() {
    assert_eq!(
        told(
            "deno",
            &[
                "adds ... ok (2ms)",
                "subtracts ... FAILED (1ms)",
                "ok | 12 passed | 0 failed (20ms)"
            ]
        ),
        [PASSED, None, None]
    );
    assert_eq!(
        told(
            "prove",
            &["ok 1 - adds", "not ok 2 - subtracts", "1..2", "okay then"]
        ),
        [PASSED, None, None, None]
    );
}

#[test]
fn scala_tests_and_commit_hooks() {
    assert_eq!(
        told(
            "sbt",
            &[
                "[info] AppSpec:",
                "[info] - should add two numbers",
                "[info] - should subtract *** FAILED ***",
                "[info] Tests: succeeded 11, failed 1, canceled 0, ignored 0, pending 0",
                "[info]   - an indented line"
            ]
        ),
        [None, PASSED, None, None, None]
    );
    assert_eq!(
        told(
            "pre-commit",
            &[
                "black....................................................................Passed",
                "check yaml...........................................(no files to check)Skipped",
                "flake8...................................................................Failed",
                "Passed",
                "- hook id: flake8"
            ]
        ),
        [Some(PASSED_HOOKS), Some(PASSED_HOOKS), None, None, None]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told(
            "go",
            &["test a ... ok", "PASS src/a.test.ts", "ok 1 - adds"]
        ),
        [None, None, None]
    );
    assert_eq!(
        told("cargo", &["=== RUN   TestAdd", "AppTest > adds() PASSED"]),
        [None, None]
    );
}
