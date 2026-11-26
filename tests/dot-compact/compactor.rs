use std::fmt::Write as _;

use proptest::prelude::*;

use super::*;

fn matches() -> String {
    (1..=30)
        .map(|line| format!("crates/dot-api/src/session.rs:{line}:    let spent = usage.cost();\n"))
        .collect::<Vec<String>>()
        .concat()
}

#[test]
fn shortens_the_output_of_a_search() {
    let shorter = ShapeCompactor::default()
        .compact(
            Origin::Command("grep -rn spent crates | head -40"),
            &matches(),
        )
        .unwrap();

    assert!(shorter.starts_with("crates/dot-api/src/session.rs\n1:    let spent"));
    assert!(shorter.len() < matches().len() * 2 / 3);
}

#[test]
fn never_touches_what_a_file_reader_prints() {
    for line in [
        "cat list.txt",
        "sed -n '1,30p' list.txt",
        "ls && head -30 list.txt",
        "git diff",
        "find . -name list.txt | xargs cat",
    ] {
        assert_eq!(
            ShapeCompactor::default().compact(Origin::Command(line), &matches()),
            None,
            "{line}"
        );
    }
}

#[test]
fn says_which_commands_keep_their_output_whole() {
    for command in [
        "DOT_RAW=1 cargo test",
        "cd crates && DOT_RAW=1 cat src/lib.rs",
        "cat /var/folders/x/T/dot-whole/40000-42",
        "sed -n '171,582p' /tmp/dot-whole/40000-42",
    ] {
        assert!(ShapeCompactor::keeps_whole(command), "{command}");
    }
    for command in [
        "cargo test",
        "grep -rn x . | head",
        "rtk git status",
        "ls -la",
        "cat src/lib.rs",
        "git diff",
        "rtk read src/lib.rs",
    ] {
        assert!(!ShapeCompactor::keeps_whole(command), "{command}");
    }
}

#[test]
fn counts_the_progress_of_a_running_command_only_when_it_knows_its_tool() {
    let progress = "   Compiling a v1\n   Compiling b v1\n   Compiling c v1\nerror: no\n";

    assert_eq!(
        ShapeCompactor::default()
            .stream("rtk cargo build")
            .unwrap()
            .push(progress),
        "[+3 lines: build progress]\nerror: no\n"
    );
    assert_eq!(
        ShapeCompactor::default()
            .stream("ls -la")
            .unwrap()
            .push(progress),
        progress
    );
}

#[test]
fn has_no_stream_for_what_passes_as_it_comes() {
    for command in [
        "cat src/lib.rs",
        "git diff",
        "tail -f app.log",
        "DOT_RAW=1 cargo build",
        "grep -n x /tmp/dot-whole/7-1",
    ] {
        assert!(
            ShapeCompactor::default().stream(command).is_none(),
            "{command}"
        );
    }
}

#[test]
fn keeps_whole_the_output_claude_asked_for_raw() {
    assert_eq!(
        ShapeCompactor::default().compact(
            Origin::Command("DOT_RAW=1 grep -rn spent crates"),
            &matches()
        ),
        None
    );
}

#[test]
fn leaves_alone_an_output_under_400_bytes() {
    let short = matches()
        .lines()
        .take(5)
        .map(|line| format!("{line}\n"))
        .collect::<Vec<String>>()
        .concat();

    assert!(short.len() < SHORTEST_WORTH_READING);
    assert_eq!(
        ShapeCompactor::default().compact(Origin::Command("grep -rn spent crates"), &short),
        None
    );
    assert!(
        ShapeCompactor::default()
            .compact(
                Origin::Command("grep -rn spent crates"),
                &format!("{short}{short}")
            )
            .is_some()
    );
}

#[test]
fn reads_an_output_of_exactly_400_bytes() {
    let found = (10..18)
        .map(|line| {
            format!(
                "src/some/long/path/file_name.rs:{line}:{}\n",
                "x".repeat(14)
            )
        })
        .collect::<Vec<String>>()
        .concat();

    assert_eq!(found.len(), 400);
    assert!(
        ShapeCompactor::default()
            .compact(Origin::Matches, &found)
            .is_some()
    );
    assert_eq!(
        ShapeCompactor::default().compact(Origin::Matches, &found[1..]),
        None
    );
}

#[test]
fn keeps_the_original_when_less_than_80_bytes_are_saved() {
    let coloured = format!("\u{1b}[32m{}\u{1b}[0m", "x".repeat(500));
    let lines = (0..10)
        .map(|line| format!("\u{1b}[32mok {line}\u{1b}[0m\n"))
        .collect::<Vec<String>>()
        .concat();
    let noisy = format!("{lines}{}", "y".repeat(400));

    assert_eq!(
        ShapeCompactor::default().compact(Origin::Command("make"), &coloured),
        None
    );
    assert_eq!(
        noisy.len() - 90,
        ShapeCompactor::default()
            .compact(Origin::Command("make"), &noisy)
            .unwrap()
            .len()
    );
}

#[test]
fn only_removes_the_colours_of_a_diff() {
    let hunk =
        "@@ -1,3 +1,3 @@\n context\n\u{1b}[31m-old line\u{1b}[m\n\u{1b}[32m+new line\u{1b}[m\n";
    let diff = format!("diff --git a/src/a.rs b/src/a.rs\n{}", hunk.repeat(12));

    let shorter = ShapeCompactor::default()
        .compact(Origin::Command("gh pr diff 12"), &diff)
        .unwrap();

    assert_eq!(
        shorter,
        diff.replace("\u{1b}[31m", "")
            .replace("\u{1b}[32m", "")
            .replace("\u{1b}[m", "")
    );
}

#[test]
fn a_diff_without_colours_stays_as_it_is() {
    let diff = format!(
        "diff --git a/a.rs b/a.rs\n{}",
        "@@ -1 +1 @@\nsrc/a.rs:1:x\nsrc/a.rs:2:x\n".repeat(20)
    );

    assert_eq!(
        ShapeCompactor::default().compact(Origin::Command("gh pr diff 12"), &diff),
        None
    );
}

#[test]
fn removes_the_indentation_of_json() {
    let pretty =
        serde_json::to_string_pretty(&serde_json::json!({"items": (0..100).collect::<Vec<u32>>()}))
            .unwrap();

    let minified = ShapeCompactor::default()
        .compact(
            Origin::Command("curl -s https://example.com/items"),
            &pretty,
        )
        .unwrap();

    assert!(minified.starts_with(r#"{"items":[0,1,2,"#));
}

#[test]
fn folds_a_list_of_files_and_counts_a_build() {
    let files = (0..40)
        .map(|file| format!("./crates/dot-desktop/src/canvas/file_{file}.rs\n"))
        .collect::<Vec<String>>()
        .concat();
    let build = (0..40)
        .map(|krate| format!("   Compiling crate_{krate} v1.0.0\n"))
        .collect::<Vec<String>>()
        .concat();

    let folded = ShapeCompactor::default()
        .compact(Origin::Command("find . -name '*.rs'"), &files)
        .unwrap();
    let counted = ShapeCompactor::default()
        .compact(
            Origin::Command("cargo build"),
            &format!("{build}    Finished in 3s\n"),
        )
        .unwrap();

    assert!(folded.starts_with("crates/dot-desktop/src/canvas/{file_0.rs,file_1.rs,"));
    assert_eq!(counted, "[+40 lines: build progress]\n    Finished in 3s\n");
}

#[test]
fn shortens_what_dots_own_tools_find() {
    let files = (0..40)
        .map(|file| format!("crates/dot-desktop/src/canvas/file_{file}.rs\n"))
        .collect::<Vec<String>>()
        .concat();

    assert!(
        ShapeCompactor::default()
            .compact(Origin::Matches, &matches())
            .is_some()
    );
    assert!(
        ShapeCompactor::default()
            .compact(Origin::Paths, &files)
            .is_some()
    );
    assert_eq!(
        ShapeCompactor::default().compact(Origin::Paths, &matches()),
        None
    );
    assert_eq!(
        ShapeCompactor::default().compact(Origin::Matches, &files),
        None
    );
}

proptest! {
    #[test]
    fn a_shorter_text_always_saves_at_least_80_bytes(line in any::<String>(), output in any::<String>()) {
        for origin in [Origin::Command(&line), Origin::Matches, Origin::Paths] {
            if let Some(shorter) = ShapeCompactor::default().compact(origin, &output) {
                prop_assert!(shorter.len() + SMALLEST_GAIN <= output.len());
            }
        }
    }

    #[test]
    fn what_a_reader_prints_is_never_rewritten(
        file in "[a-z/._]{1,20}",
        line in "[ -~]{0,60}",
        lines in 0_usize..500,
    ) {
        let output = format!("{line}\n").repeat(lines);
        let read = ShapeCompactor::default().compact(Origin::Command(&format!("cat {file}")), &output);

        match read {
            None => prop_assert!(output.len() <= 10_000 + cut::LEAST_WORTH_A_CUT),
            Some(cut) => {
                prop_assert!(output.len() > 10_000);
                for kept in cut.lines().filter(|kept| !kept.starts_with('[')) {
                    prop_assert_eq!(kept, line.as_str());
                }
            }
        }
    }
}

const CURL_METER: &str = "  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current\n                                 Dload  Upload   Total   Spent    Left  Speed\n\r  0     0    0     0    0     0      0      0 --:--:-- --:--:-- --:--:--     0\r100    51  100    51    0     0   133k      0 --:--:-- --:--:-- --:--:-- 51000\n";
const SHELLCHECK: &str = "\nIn probe.sh line 2:\necho $foo\n     ^--^ SC2154 (warning): foo is referenced but not assigned.\n     ^--^ SC2086 (info): Double quote to prevent globbing and word splitting.\n\nDid you mean:\necho \"$foo\"\n\n\nIn probe.sh line 3:\ncd $1\n^---^ SC2164 (warning): Use 'cd ... || exit' or 'cd ... || return' in case cd fails.\n   ^-- SC2086 (info): Double quote to prevent globbing and word splitting.\n\nDid you mean:\ncd \"$1\" || exit\n\nFor more information:\n  https://www.shellcheck.net/wiki/SC2045 -- Iterating over ls output is fragi...\n  https://www.shellcheck.net/wiki/SC2115 -- Use \"${var:?}\" to ensure this nev...\n  https://www.shellcheck.net/wiki/SC2154 -- dir is referenced but not assigned.\n";

fn numbered(last: usize, line: impl Fn(usize) -> String) -> String {
    (1..=last).map(line).collect::<Vec<String>>().concat()
}

fn shortened(command: &str, output: &str) -> String {
    ShapeCompactor::default()
        .compact(Origin::Command(command), output)
        .unwrap_or_else(|| panic!("{command} keeps its output"))
}

#[test]
fn counts_the_meter_of_curl_and_removes_the_indentation_of_what_it_fetched() {
    let items = numbered(40, |item| format!("    {item},\n"));
    let output =
        format!("{CURL_METER}{{\n  \"name\": \"dot\",\n  \"items\": [\n{items}    41\n  ]\n}}\n");

    let shorter = shortened("curl https://api.example.com/items", &output);

    assert!(
        shorter.starts_with("[+3 lines: transfer progress]\n{\"name\":\"dot\",\"items\":[1,2,")
    );
    assert!(shorter.ends_with(",40,41]}"));
}

#[test]
fn counts_the_links_under_what_shellcheck_found() {
    let shorter = shortened("shellcheck probe.sh", SHELLCHECK);

    assert!(shorter.starts_with("\nIn probe.sh line 2:\necho $foo\n     ^--^ SC2154 (warning)"));
    assert!(shorter.ends_with("cd \"$1\" || exit\n\n[+4 lines: links to the rules]\n"));
}

#[test]
fn counts_the_checks_of_a_pull_request_that_passed() {
    let passed = numbered(12, |job| {
        format!(
            "build ({job})\tpass\t5m25s\thttps://github.com/cli/cli/actions/runs/44584003105/job/{job}\t\n"
        )
    });
    let failed = "lint\tfail\t1m2s\thttps://github.com/cli/cli/actions/runs/44584003105/job/13\t\n";

    assert_eq!(
        shortened("gh pr checks 15109", &format!("{passed}{failed}")),
        format!("[+12 lines: checks that passed]\n{failed}")
    );
}

#[test]
fn groups_what_a_type_checker_found_under_each_file() {
    let errors = numbered(10, |line| {
        format!(
            "src/components/billing/invoice.tsx({line},5): error TS2322: Type 'string' is not assignable to type 'number'.\n"
        )
    });

    let shorter = shortened("npx tsc --noEmit", &errors);

    assert!(shorter.starts_with(
        "src/components/billing/invoice.tsx\n(1,5): error TS2322: Type 'string' is not assignable to type 'number'.\n(2,5): "
    ));
    assert_eq!(shorter.matches("invoice.tsx").count(), 1);
}

#[test]
fn rewrites_a_tree_a_table_and_a_list_of_counted_files() {
    let tree = numbered(30, |file| {
        format!("│   │   ├── a_rather_long_file_name_{file}.rs\n")
    });
    let table = numbered(20, |id| {
        format!(" {id:>3} | a customer with a name   | {id:>10} \n")
    });
    let sizes = numbered(20, |file| {
        format!("{file:>8} crates/dot-compact/src/report/rules/file_{file}.rs\n")
    });

    let tree = shortened(
        "tree crates",
        &format!(".\n├── crates\n│   ├── dot\n{tree}"),
    );
    let table = shortened(
        "psql -c 'select * from customers'",
        &format!(
            "  id |           name           |   amount   \n-----+--------------------------+------------\n{table}(20 rows)\n"
        ),
    );
    let sizes = shortened("wc -l crates/dot-compact/src/report/rules/*.rs", &sizes);

    assert!(tree.starts_with(
        "[tree as: one space per level]\n.\n crates\n  dot\n   a_rather_long_file_name_1.rs\n"
    ));
    assert!(table.starts_with("id|name|amount\n1|a customer with a name|1\n2|"));
    assert!(table.ends_with("20|a customer with a name|20\n(20 rows)\n"));
    assert!(sizes.starts_with(
        "[paths under crates/dot-compact/src/report/rules/]\n1 file_1.rs\n2 file_2.rs\n"
    ));
}

#[test]
fn shortens_the_rulers_and_the_marks_a_tool_draws() {
    let diagnostics = numbered(4, |file| {
        format!(
            "./src/app_{file}.ts:3:7 lint/style/useConst  FIXABLE  {}\n\n  ✖ This let declares a variable that is only assigned once.\n\n",
            "━".repeat(60)
        )
    });
    let examples = format!(
        "Randomized with seed 4242\n\n{}F\n\nFinished in 12.4 seconds\n500 examples, 1 failure\n",
        ".".repeat(499)
    );

    let diagnostics = shortened("npx biome check", &diagnostics);
    let examples = shortened("bundle exec rspec", &examples);

    assert!(
        diagnostics
            .starts_with("./src/app_1.ts:3:7 lint/style/useConst  FIXABLE  ━━━\n\n  ✖ This let")
    );
    assert_eq!(
        examples,
        "Randomized with seed 4242\n\n[+500 marks: 499 . 1 F]\n\nFinished in 12.4 seconds\n500 examples, 1 failure\n"
    );
}

fn log_of(commits: usize) -> String {
    numbered(commits, |commit| {
        format!(
            "commit dbe543bc0d2fc10a06a3b4c2c6cb6e0945c0c2{commit:02}\nAuthor: Jules <jules@example.com>\nDate:   Thu Nov 19 18:{commit:02}:00 2026 +0100\n\n    test(desktop): pin the screen switch\n\n"
        )
    })
}

#[test]
fn writes_the_author_of_a_log_once_and_folds_the_names_of_changed_files() {
    let names = numbered(12, |file| {
        format!("crates/dot-model/src/claude_code/launch/file_{file}.rs\n")
    });

    let log = shortened("git log -8", &log_of(8));
    let names = shortened("git diff --name-only", &names);

    assert!(log.starts_with(
        "[git log: every commit by Jules <jules@example.com>]\ncommit dbe543bc0d2fc10a06a3b4c2c6cb6e0945c0c201\nDate:   Thu Nov 19 18:01:00 2026 +0100\n    test(desktop): pin the screen switch\ncommit "
    ));
    assert_eq!(log.matches("Author:").count(), 0);
    assert!(!log.contains("[cut:"));
    assert!(names.starts_with("crates/dot-model/src/claude_code/launch/{file_1.rs,file_2.rs,"));
}

#[test]
fn writes_a_log_too_long_to_keep_one_line_per_commit_and_says_it_cut() {
    let log = shortened("git log -60", &log_of(60));

    assert!(log.starts_with(
        "[git log: every commit by Jules <jules@example.com>]\n[git log as: hash day subject, one line per commit]\ndbe543b Nov 19 2026 test(desktop): pin the screen switch\ndbe543b Nov 19 2026 "
    ));
    assert_eq!(log.lines().count(), 63);
    assert!(log.ends_with(&format!("{}\n", dot_api::CUT_OUTPUT_NOTE)));
}

#[test]
fn never_touches_what_sops_decrypts_or_a_diff_shows() {
    for line in [
        "sops -d secrets.enc.yaml",
        "git diff",
        "git diff --name-only -p",
    ] {
        assert_eq!(
            ShapeCompactor::default().compact(Origin::Command(line), &matches()),
            None,
            "{line}"
        );
        assert!(ShapeCompactor::default().stream(line).is_none(), "{line}");
    }
    assert!(
        ShapeCompactor::default()
            .stream("git diff --name-only")
            .is_some()
    );
}

fn file_of(lines: usize) -> String {
    (1..=lines)
        .map(|line| format!("    let value_{line:04} = compute({line});\n"))
        .collect::<Vec<String>>()
        .concat()
}

#[test]
fn cuts_the_middle_of_a_long_file_and_says_how_to_read_it_whole() {
    let file = file_of(400);

    let shorter = ShapeCompactor::default()
        .compact(Origin::Command("cat src/big.rs"), &file)
        .unwrap();

    assert!(file.len() > 10_000);
    assert!(shorter.starts_with("    let value_0001 = compute(1);\n"));
    assert!(shorter.contains(" lines cut: "));
    assert!(shorter.ends_with(&format!(
        "    let value_0400 = compute(400);\n{}\n",
        dot_api::CUT_OUTPUT_NOTE
    )));
    assert!(shorter.len() < 6_300);
    for kept in shorter.lines().filter(|kept| !kept.starts_with('[')) {
        assert!(file.contains(&format!("{kept}\n")), "{kept}");
    }
    assert_eq!(
        ShapeCompactor::default().compact(Origin::Command("cat src/small.rs"), &file_of(250)),
        None
    );
}

#[test]
fn never_cuts_what_claude_asked_for_whole_nor_what_dots_own_tools_find() {
    let file = file_of(400);

    for command in [
        "DOT_RAW=1 cat src/big.rs",
        "cat /var/folders/x/T/dot-whole/40000-42",
        "grep -n value /tmp/dot-whole/40000-42",
    ] {
        assert_eq!(
            ShapeCompactor::default().compact(Origin::Command(command), &file),
            None,
            "{command}"
        );
    }
    for origin in [Origin::Matches, Origin::Paths] {
        let found = ShapeCompactor::default().compact(origin, &file);
        assert!(found.is_none_or(|found| !found.contains("lines cut")));
    }
}

#[test]
fn cuts_a_long_output_once_shortened_and_a_long_diff_once_its_colours_are_gone() {
    let log = (1..=400)
        .map(|request| {
            format!("\u{1b}[2mrequest {request:04}\u{1b}[0m handled, nothing to report\n")
        })
        .collect::<Vec<String>>()
        .concat();
    let diff = format!(
        "diff --git a/a.rs b/a.rs\n{}",
        "@@ -1,3 +1,3 @@\n context\n\u{1b}[31m-old line\u{1b}[m\n+new line\n".repeat(300)
    );
    let note = format!("{}\n", dot_api::CUT_OUTPUT_NOTE);

    let cut_log = shortened("./serve --once", &log);
    let cut_diff = shortened("gh pr diff 12", &diff);

    assert!(cut_log.starts_with("request 0001 handled, nothing to report\n"));
    assert!(cut_log.ends_with(&format!("request 0400 handled, nothing to report\n{note}")));
    assert!(cut_log.len() < 3_300);
    assert!(
        cut_diff.starts_with("diff --git a/a.rs b/a.rs\n@@ -1,3 +1,3 @@\n context\n-old line\n")
    );
    assert!(cut_diff.ends_with(&format!("+new line\n{note}")));
    assert!(cut_diff.contains("; among them, these open a block]\n354:@@ -1,3 +1,3 @@\n"));
    assert!(cut_diff.len() < 7_300 && !cut_diff.contains('\u{1b}'));
}

#[test]
fn keeps_the_start_of_a_long_json_document_and_the_whole_of_a_shorter_one() {
    let long = serde_json::to_string_pretty(
        &serde_json::json!({"items": (0..3_000).collect::<Vec<u32>>()}),
    )
    .unwrap();
    let short =
        serde_json::to_string_pretty(&serde_json::json!({"items": (0..900).collect::<Vec<u32>>()}))
            .unwrap();

    let cut = shortened("curl -s https://example.com/items", &long);
    let whole = shortened("curl -s https://example.com/items", &short);

    assert!(cut.starts_with(r#"{"items":[0,1,2,"#));
    assert!(cut.ends_with(&format!(" chars]\n{}", dot_api::CUT_OUTPUT_NOTE)));
    assert!(cut.len() < 3_200);
    assert!(whole.ends_with(",898,899]}") && whole.len() < 5_000);
}

#[test]
fn keeps_as_much_of_a_compact_json_document_as_of_the_same_one_indented() {
    let items = serde_json::json!({"items": (0..3_000).collect::<Vec<u32>>()});
    let compact = serde_json::to_string(&items).unwrap();
    let indented = serde_json::to_string_pretty(&items).unwrap();

    let cut = shortened("curl -s https://example.com/items", &compact);

    assert_eq!(
        cut,
        shortened("curl -s https://example.com/items", &indented)
    );
    assert!(cut.starts_with(r#"{"items":[0,1,2,"#));
    assert!(cut.len() > 3_000 && cut.len() < 3_200);
}

#[test]
fn leaves_whole_a_compact_json_document_too_short_to_cut() {
    let items =
        serde_json::to_string(&serde_json::json!({"items": (0..900).collect::<Vec<u32>>()}))
            .unwrap();
    assert!(items.len() > 3_000 && items.len() < 7_000);

    let told = ShapeCompactor::default()
        .compact(Origin::Command("curl -s https://example.com/items"), &items);

    assert_eq!(told, None);
}

proptest! {
    #[test]
    fn a_compact_json_document_reads_as_the_same_one_indented(
        items in prop::collection::vec(any::<u32>(), 0..3_000),
    ) {
        let document = serde_json::json!({ "items": items });
        let compact = serde_json::to_string(&document).unwrap();
        let indented = serde_json::to_string_pretty(&document).unwrap();
        let command = "curl -s https://example.com/items";

        let compact_told = ShapeCompactor::default()
            .compact(Origin::Command(command), &compact)
            .unwrap_or_else(|| compact.clone());
        let indented_told = ShapeCompactor::default()
            .compact(Origin::Command(command), &indented)
            .unwrap_or_else(|| indented.clone());

        prop_assert!(indented_told == indented || indented_told == compact_told);
    }
}

#[test]
fn makes_a_terminal_output_readable_without_its_colours_and_earlier_redraws() {
    let printed = "\u{1b}[32mok\u{1b}[0m 1\n 10%\r 50%\r100%\ndone";

    let readable = ShapeCompactor::default().readable(printed);

    assert_eq!(readable, "ok 1\n100%\ndone");
}

#[test]
fn leaves_a_plain_output_as_it_is_without_a_copy() {
    let printed = "compiling dot\nfinished\n";

    let readable = ShapeCompactor::default().readable(printed);

    assert!(matches!(readable, Cow::Borrowed(_)));
    assert_eq!(readable, printed);
}

fn with(compaction: OutputCompaction) -> ShapeCompactor {
    ShapeCompactor::new(compaction)
}

const NOTHING: OutputCompaction = OutputCompaction {
    cleaning: dot_api::Toggle::Off,
    rewriting: dot_api::Toggle::Off,
    progress: dot_api::Toggle::Off,
    contents_cut: dot_api::CutLevel::Never,
    outputs_cut: dot_api::CutLevel::Never,
};

fn source_of(bytes: usize) -> String {
    let mut text = String::new();
    let mut line = 0;
    while text.len() < bytes {
        line += 1;
        writeln!(
            text,
            "    let value_{line} = compute({line}, \"some text\");"
        )
        .unwrap();
    }
    text
}

fn long_unknown_output(bytes: usize) -> String {
    const WORDS: [&str; 8] = [
        "alpha", "bravo", "delta", "echo", "golf", "hotel", "kilo", "lima",
    ];
    let mut text = String::new();
    let mut seed: usize = 7;
    while text.len() < bytes {
        let line: Vec<&str> = (0..6)
            .map(|_| {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                WORDS[(seed >> 33) % WORDS.len()]
            })
            .collect();
        text.push_str(&line.join(" "));
        text.push('\n');
    }
    text
}

fn coloured_build() -> String {
    let mut text = String::new();
    for crate_number in 0..12 {
        writeln!(
            text,
            "   \u{1b}[1m\u{1b}[32mCompiling\u{1b}[0m crate-{crate_number} v1.0.0 (/somewhere/long/enough/to/count)"
        )
        .unwrap();
    }
    text.push_str("    Finished `dev` profile in 2.1s\n");
    text
}

#[test]
fn does_nothing_at_all_with_every_part_off() {
    let nothing = with(NOTHING);

    assert_eq!(
        nothing.compact(Origin::Command("grep -rn spent crates"), &matches()),
        None
    );
    assert_eq!(nothing.compact(Origin::Matches, &matches()), None);
    assert_eq!(
        nothing.compact(Origin::Command("cargo build"), &coloured_build()),
        None
    );
    assert_eq!(
        nothing.compact(Origin::Command("cat big.rs"), &source_of(40_000)),
        None
    );
    assert_eq!(
        nothing.compact(Origin::Command("./job"), &long_unknown_output(40_000)),
        None
    );
    assert!(nothing.stream("cargo build").is_none());
    assert!(nothing.burst("./job").is_none());
    assert!(nothing.burst("cat big.rs").is_none());
}

#[test]
fn leaves_the_colours_when_cleaning_is_off() {
    let uncleaned = with(OutputCompaction {
        cleaning: dot_api::Toggle::Off,
        ..OutputCompaction::FULL
    });

    let counted = uncleaned
        .compact(Origin::Command("cargo build"), &coloured_build())
        .unwrap_or_else(coloured_build);
    let cleaned = ShapeCompactor::default()
        .compact(Origin::Command("cargo build"), &coloured_build())
        .unwrap();

    assert!(counted.contains('\u{1b}'));
    assert!(!cleaned.contains('\u{1b}'));
}

#[test]
fn removes_only_the_colours_with_cleaning_alone() {
    let cleaning = with(OutputCompaction {
        cleaning: dot_api::Toggle::On,
        ..NOTHING
    });

    let cleaned = cleaning
        .compact(Origin::Command("cargo build"), &coloured_build())
        .unwrap();

    assert!(!cleaned.contains('\u{1b}'));
    assert_eq!(cleaned.lines().count(), coloured_build().lines().count());
}

#[test]
fn rewrites_nothing_when_rewriting_is_off() {
    let unrewritten = with(OutputCompaction {
        rewriting: dot_api::Toggle::Off,
        ..OutputCompaction::FULL
    });
    let paths: String = (0..40).fold(String::new(), |mut paths, file| {
        writeln!(paths, "./src/module/file_{file}.rs").unwrap();
        paths
    });

    assert_eq!(unrewritten.compact(Origin::Matches, &matches()), None);
    assert_eq!(unrewritten.compact(Origin::Paths, &paths), None);
    assert_eq!(
        unrewritten.compact(Origin::Command("find . -name '*.rs'"), &paths),
        None
    );
    assert!(
        ShapeCompactor::default()
            .compact(Origin::Paths, &paths)
            .is_some()
    );
}

#[test]
fn keeps_indented_json_whole_when_rewriting_is_off() {
    let json = format!(
        "{{\n{}\n}}\n",
        (0..40)
            .map(|key| format!("    \"key_{key}\": \"value\""))
            .collect::<Vec<_>>()
            .join(",\n")
    );
    let unrewritten = with(OutputCompaction {
        rewriting: dot_api::Toggle::Off,
        ..OutputCompaction::FULL
    });

    assert_eq!(
        unrewritten.compact(Origin::Command("curl -s api"), &json),
        None
    );
    assert!(
        ShapeCompactor::default()
            .compact(Origin::Command("curl -s api"), &json)
            .is_some()
    );
}

#[test]
fn counts_no_progress_when_counting_is_off() {
    let uncounted = with(OutputCompaction {
        progress: dot_api::Toggle::Off,
        ..OutputCompaction::FULL
    });
    let build = "   Compiling a v1.0.0 (/a/somewhere/rather/long/to/make/the/line/wider)\n"
        .repeat(12)
        + "    Finished `dev` profile in 2.1s\n";

    let counted = ShapeCompactor::default().compact(Origin::Command("cargo build"), &build);
    assert!(counted.is_some_and(|counted| counted.contains("build progress")));
    assert!(
        uncounted
            .compact(Origin::Command("cargo build"), &build)
            .is_none_or(|told| !told.contains("build progress"))
    );
}

#[test]
fn cuts_the_contents_of_a_file_at_the_level_chosen() {
    let at = |level| {
        with(OutputCompaction {
            contents_cut: level,
            ..OutputCompaction::FULL
        })
        .compact(Origin::Command("cat big.rs"), &source_of(12_000))
    };

    assert_eq!(at(dot_api::CutLevel::Never), None);
    assert_eq!(at(dot_api::CutLevel::Late), None);
    assert!(at(dot_api::CutLevel::Measured).is_some_and(|cut| cut.len() < 7_000));
    assert!(at(dot_api::CutLevel::Early).is_some_and(|cut| cut.len() < 5_000));
}

#[test]
fn cuts_the_contents_of_a_file_late_only_above_13_000_bytes() {
    let late = with(OutputCompaction {
        contents_cut: dot_api::CutLevel::Late,
        ..OutputCompaction::FULL
    });

    let cut = late
        .compact(Origin::Command("cat big.rs"), &source_of(20_000))
        .unwrap();

    assert!(cut.len() > 9_000 && cut.len() < 11_000, "{}", cut.len());
}

#[test]
fn cuts_other_outputs_at_the_level_chosen() {
    let output = long_unknown_output(12_000);
    let at = |level| {
        with(OutputCompaction {
            outputs_cut: level,
            ..OutputCompaction::FULL
        })
        .compact(Origin::Command("./job"), &output)
        .map(|cut| cut.len())
    };

    assert_eq!(at(dot_api::CutLevel::Never), None);
    let late = at(dot_api::CutLevel::Late).unwrap();
    let measured = at(dot_api::CutLevel::Measured).unwrap();
    let early = at(dot_api::CutLevel::Early).unwrap();
    assert!(
        late > measured && measured > early,
        "{late} {measured} {early}"
    );
    assert!(early < 3_000, "{early}");
}

#[test]
fn a_file_cut_never_leaves_other_outputs_cut() {
    let files_never = with(OutputCompaction {
        contents_cut: dot_api::CutLevel::Never,
        ..OutputCompaction::FULL
    });

    assert_eq!(
        files_never.compact(Origin::Command("cat big.rs"), &source_of(40_000)),
        None
    );
    assert!(
        files_never
            .compact(Origin::Command("./job"), &long_unknown_output(40_000))
            .is_some_and(|cut| cut
                .ends_with("[cut: start the command with DOT_RAW=1 to read its output whole]\n"))
    );
}

#[test]
fn streams_while_either_cleaning_or_counting_is_on() {
    let only = |cleaning, progress| {
        with(OutputCompaction {
            cleaning,
            progress,
            ..NOTHING
        })
        .stream("cargo build")
        .is_some()
    };

    assert!(only(dot_api::Toggle::On, dot_api::Toggle::Off));
    assert!(only(dot_api::Toggle::Off, dot_api::Toggle::On));
    assert!(!only(dot_api::Toggle::Off, dot_api::Toggle::Off));
}

#[test]
fn bursts_within_the_bounds_of_the_level_chosen() {
    let pieces = long_unknown_output(30_000);
    let told = |level| {
        let mut burst = with(OutputCompaction {
            outputs_cut: level,
            ..OutputCompaction::FULL
        })
        .burst("./job")
        .unwrap();
        let mut told = burst.pass(pieces.as_bytes());
        told.extend(burst.pause());
        told.len()
    };

    let late = told(dot_api::CutLevel::Late);
    let early = told(dot_api::CutLevel::Early);
    assert!(late > early, "{late} {early}");
    assert!(late < 6_000 && early < 3_000, "{late} {early}");
}

#[test]
fn tunes_to_new_choices() {
    let mut compactor = ShapeCompactor::default();

    compactor.tune(NOTHING);

    assert_eq!(compactor.compact(Origin::Matches, &matches()), None);
}
