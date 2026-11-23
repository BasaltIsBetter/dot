use super::*;

fn prints_content(line: &str) -> bool {
    CommandLine::read(line).prints_content()
}

#[test]
fn what_a_file_reader_prints_is_content() {
    for line in [
        "cat src/lib.rs",
        "sed -n '1,50p' src/lib.rs",
        "cd crates && head -3 Cargo.toml",
        "ls -la && cat notes.txt",
        r"find . -name '*.md' -exec cat {} \;",
        "find . -name '*.md' | xargs cat",
        "for f in a b; do tail -5 $f; done",
        "git diff",
        "git -C repo show HEAD:src/lib.rs",
        "git log -p -3",
        "git blame src/lib.rs",
    ] {
        assert!(prints_content(line), "{line}");
    }
}

#[test]
fn what_filters_a_pipe_or_edits_in_place_is_not_content() {
    for line in [
        "grep -rn foo src | head -20",
        "ls -la | sort -k5",
        "find . -type f | sed 's#^./##' | sort",
        "sed -i '' 's/a/b/' src/lib.rs && cargo build",
        "git status",
        "git log --oneline -5",
        "cargo test 2>&1 | tail -30",
        "ps aux | awk '{print $2}'",
    ] {
        assert!(!prints_content(line), "{line}");
    }
}

#[test]
fn reads_through_the_prefix_of_another_filter() {
    assert!(prints_content("rtk read src/lib.rs"));
    assert!(prints_content("rtk proxy cat src/lib.rs"));
    assert!(!prints_content("rtk grep -rn foo src"));
    assert!(CommandLine::read("rtk git status").runs_with("git", "status"));
    assert!(CommandLine::read("rtk cargo test").mentions("cargo"));
}

#[test]
fn the_raw_mark_is_a_word_of_the_line() {
    assert!(CommandLine::read("DOT_RAW=1 cargo test").keeps_raw());
    assert!(CommandLine::read("cd crates && DOT_RAW=1 ls -la").keeps_raw());
    assert!(!CommandLine::read("echo DOT_RAW=10").keeps_raw());
    assert!(CommandLine::read("cat /tmp/dot-whole/40000-42").keeps_raw());
    assert!(CommandLine::read("grep -c x '/var/T/dot-whole/7-1' | wc -l").keeps_raw());
    assert!(!CommandLine::read("cat /tmp/dot-who/1795-42").keeps_raw());
    assert!(!CommandLine::read("cargo test").keeps_raw());
}

#[test]
fn a_tool_is_mentioned_as_a_program_or_as_an_argument() {
    assert!(CommandLine::read("./gradlew :core:test").mentions("gradlew"));
    assert!(CommandLine::read("python3 -m pytest tests").mentions("pytest"));
    assert!(CommandLine::read("npx vitest run").mentions("vitest"));
    assert!(CommandLine::read("node node_modules/.bin/jest").mentions("jest"));
    assert!(!CommandLine::read("echo cargos").mentions("cargo"));
}

#[test]
fn a_program_runs_with_an_argument_wherever_the_argument_stands() {
    assert!(CommandLine::read("git status").runs_with("git", "status"));
    assert!(CommandLine::read("git -C repo status --long").runs_with("git", "status"));
    assert!(!CommandLine::read("git log").runs_with("git", "status"));
    assert!(!CommandLine::read("echo status").runs_with("git", "status"));
}

#[test]
fn what_sops_decrypts_is_content() {
    assert!(prints_content("sops -d secrets.enc.yaml"));
    assert!(prints_content("cd deploy && sops decrypt secrets.enc.yaml"));
}

#[test]
fn the_names_of_the_changed_files_are_not_content() {
    for line in [
        "git diff --name-only",
        "git diff --name-only HEAD~3",
        "git -C repo show --name-only HEAD",
    ] {
        assert!(!prints_content(line), "{line}");
    }
    for line in [
        "git diff --name-only -p",
        "git show --patch --name-only",
        "git diff --stat",
        "git diff",
    ] {
        assert!(prints_content(line), "{line}");
    }
}

#[test]
fn what_a_reader_prints_through_another_program_is_content() {
    for line in [
        "ssh build-box 'cat /srv/app/src/lib.rs'",
        r#"ssh -o BatchMode=yes me@host "cd /srv && sed -n '1,40p' app.conf""#,
        "ssh host tail -n 50 /var/log/app.log",
        "docker exec web cat /app/config.yml",
        "kubectl exec pod -- head -20 /etc/app.conf",
        "sh -c 'ls; cat notes.txt'",
        r#"bash -lc "git diff""#,
        "timeout 30s cat big.log",
        "watch cat notes.txt",
    ] {
        assert!(prints_content(line), "{line}");
    }
    for carrier in CARRIERS {
        assert!(
            prints_content(&format!("{carrier} x cat notes.txt")),
            "{carrier}"
        );
        assert!(
            prints_content(&format!("{carrier} x 'cat notes.txt'")),
            "{carrier}"
        );
    }
}

#[test]
fn what_another_program_runs_without_a_reader_is_not_content() {
    for line in [
        "ssh host 'ls -la /srv'",
        r#"ssh host "grep -rn foo /etc | head -20""#,
        "docker exec web sed -i s/a/b/ /app/config.yml",
        "docker logs --tail 50 web",
        "kubectl logs pod --tail=50",
        "sh -c 'cargo build'",
        "ssh host 'unterminated cat",
        "ssh host cat' notes.txt'",
        "echo 'cat is not run here'",
        "grep -rn 'head of' src",
        "grep -rn head src",
    ] {
        assert!(!prints_content(line), "{line}");
    }
}
