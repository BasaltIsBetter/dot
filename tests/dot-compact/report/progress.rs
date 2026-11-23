use super::*;

fn compacted(line: &str, output: &str) -> Option<String> {
    compact(&CommandLine::read(line), output)
}

#[test]
fn counts_the_steps_of_a_cargo_build() {
    let output = "    Updating crates.io index\n   Compiling a v1.0.0\n    Checking b v1.0.0\n   Compiling c v1.0.0 (/path)\nwarning: unused variable\n    Finished `dev` profile in 2.1s\n";

    assert_eq!(
        compacted("cargo build", output).as_deref(),
        Some(
            "[+4 lines: build progress]\nwarning: unused variable\n    Finished `dev` profile in 2.1s\n"
        )
    );
}

#[test]
fn counts_the_tests_that_passed_and_keeps_the_others() {
    let output = "running 5 tests\ntest a ... ok\ntest b ... ok\ntest c ... ok\ntest d ... ignored\ntest e ... FAILED\ntest result: FAILED. 3 passed; 1 failed; 1 ignored\n";

    assert_eq!(
        compacted("cargo test -p dot-api", output).as_deref(),
        Some(
            "running 5 tests\n[+3 lines: tests that passed]\ntest d ... ignored\ntest e ... FAILED\ntest result: FAILED. 3 passed; 1 failed; 1 ignored\n"
        )
    );
}

#[test]
fn keeps_a_run_too_short_to_be_worth_a_count() {
    assert_eq!(
        compacted(
            "cargo build",
            "   Compiling a v1.0.0\n   Compiling b v1.0.0\n    Finished\n"
        ),
        None
    );
}

#[test]
fn only_drops_lines_when_the_command_names_the_tool() {
    let output = "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n";

    assert!(compacted("cargo check", output).is_some());
    assert_eq!(compacted("make all", output), None);
    assert_eq!(compacted("./gradlew build", output), None);
}

#[test]
fn does_not_drop_a_line_that_only_looks_like_a_step() {
    let output = "Compiling notes for the release\nCompiling a list\nCompiling again\ntest plan ... ok maybe\ntest the thing\ntest more\n";

    assert_eq!(compacted("cargo run", output), None);
}

#[test]
fn drops_gradle_tasks_that_report_nothing_and_keeps_those_that_do() {
    let output = "Starting a Gradle Daemon (subsequent builds will be faster)\n> Task :api:compileJava UP-TO-DATE\n> Task :api:classes UP-TO-DATE\n> Task :api:jar\n> Task :core:compileJava\nwarning: [deprecation] old() is deprecated\n> Task :core:classes\n> Task :core:jar\n> Task :core:assemble\n\nBUILD SUCCESSFUL in 4s\n7 actionable tasks: 4 executed, 3 up-to-date\n";

    assert_eq!(
        compacted("./gradlew build", output).as_deref(),
        Some(
            "Starting a Gradle Daemon (subsequent builds will be faster)\n[+3 lines: tasks with nothing to report]\n> Task :core:compileJava\nwarning: [deprecation] old() is deprecated\n[+3 lines: tasks with nothing to report]\n\nBUILD SUCCESSFUL in 4s\n7 actionable tasks: 4 executed, 3 up-to-date\n"
        )
    );
}

#[test]
fn counts_what_git_says_while_it_transfers() {
    let output = "remote: Enumerating objects: 9, done.\nremote: Counting objects: 100% (9/9), done.\nremote: Compressing objects: 100% (5/5), done.\nremote: Total 5 (delta 3)\nTo github.com:me/dot.git\n   305a7ed..dbe543b  main -> main\n";

    assert_eq!(
        compacted("git push", output).as_deref(),
        Some(
            "[+4 lines: transfer progress]\nTo github.com:me/dot.git\n   305a7ed..dbe543b  main -> main\n"
        )
    );
}

#[test]
fn counts_the_files_whose_python_tests_all_passed() {
    let output = "collected 9 items\n\ntests/test_a.py ...                    [ 33%]\ntests/test_b.py .s.                    [ 66%]\ntests/test_c.py ...                    [100%]\ntests/test_d.py .F.                    [100%]\n\n9 passed in 0.12s\n";

    assert_eq!(
        compacted("python3 -m pytest", output).as_deref(),
        Some(
            "collected 9 items\n\n[+3 lines: tests that passed]\ntests/test_d.py .F.                    [100%]\n\n9 passed in 0.12s\n"
        )
    );
}

#[test]
fn counts_install_and_download_lines() {
    let pip = "Requirement already satisfied: a in ./venv\nCollecting b\n  Downloading b-1.0.whl (12 kB)\nInstalling collected packages: b\n";
    let maven = "[INFO] Downloading from central: https://repo/a.pom\n[INFO] Downloaded from central: https://repo/a.pom (2 kB)\n[INFO] Downloading from central: https://repo/a.jar\n[INFO] BUILD SUCCESS\n";
    let vitest = " ✓ src/a.test.ts (3)\n ✓ src/b.test.ts (2)\n ✓ src/c.test.ts (1)\n Test Files  3 passed (3)\n";

    assert_eq!(
        compacted("pip install -r requirements.txt", pip).as_deref(),
        Some("[+3 lines: install progress]\nInstalling collected packages: b\n")
    );
    assert_eq!(
        compacted("mvn -q package", maven).as_deref(),
        Some("[+3 lines: transfer progress]\n[INFO] BUILD SUCCESS\n")
    );
    assert_eq!(
        compacted("npx vitest run", vitest).as_deref(),
        Some("[+3 lines: tests that passed]\n Test Files  3 passed (3)\n")
    );
}

#[test]
fn reaches_a_tool_through_a_wrapper_a_script_or_a_compound_command() {
    let marks = " ✓ src/a.test.ts (3)\n ✓ src/b.test.ts (2)\n ✓ src/c.test.ts (1)\n Test Files  3 passed (3)\n";
    let verbose = "tests/test_a.py::test_one PASSED [ 33%]\ntests/test_a.py::test_two PASSED [ 66%]\ntests/test_a.py::test_three PASSED [100%]\n3 passed in 0.01s\n";
    let go = "=== RUN   TestAdd\n--- PASS: TestAdd (0.00s)\n=== RUN   TestSub\n--- PASS: TestSub (0.00s)\nPASS\nok  \texample.com/app\t0.2s\n";
    let build =
        "#5 [2/7] RUN make\n#5 0.1 cc -c a.c\n#5 0.2 cc -c b.c\n#5 DONE 1.2s\n#6 [3/7] COPY . .\n";

    assert_eq!(
        compacted("npm test", marks).as_deref(),
        Some("[+3 lines: checks that passed]\n Test Files  3 passed (3)\n")
    );
    assert_eq!(
        compacted("cd web && pnpm run test -- --run", marks).as_deref(),
        Some("[+3 lines: checks that passed]\n Test Files  3 passed (3)\n")
    );
    assert_eq!(
        compacted("uv run pytest -v", verbose).as_deref(),
        Some("[+3 lines: tests that passed]\n3 passed in 0.01s\n")
    );
    assert_eq!(
        compacted("go test -v ./... 2>&1 | tail -40", go).as_deref(),
        Some("[+4 lines: tests that passed]\nPASS\nok  \texample.com/app\t0.2s\n")
    );
    assert_eq!(
        compacted("docker build -t app .", build).as_deref(),
        Some("#5 [2/7] RUN make\n[+3 lines: image build progress]\n#6 [3/7] COPY . .\n")
    );
    assert_eq!(compacted("make test", marks), None);
}
