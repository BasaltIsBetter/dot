use super::*;

fn change(path: &str) -> Action {
    Action::Change {
        path: path.to_owned(),
    }
}

fn run(command: &str) -> Action {
    Action::Run {
        command: command.to_owned(),
    }
}

#[test]
fn asks_everything_until_the_user_says_always() {
    let policy = Policy::new(PermissionMode::Ask);

    assert_eq!(policy.judge(&change("src/main.rs")), Verdict::Ask);
    assert_eq!(policy.judge(&run("ls")), Verdict::Ask);
}

#[test]
fn bypass_allows_everything_protected_files_included() {
    let policy = Policy::new(PermissionMode::Bypass);

    assert_eq!(policy.judge(&run("rm -rf target")), Verdict::Allow);
    assert_eq!(
        policy.judge(&change(".git/hooks/pre-commit")),
        Verdict::Allow
    );
}

#[test]
fn always_covers_what_the_action_grants_and_nothing_more() {
    let mut policy = Policy::new(PermissionMode::Ask);

    policy.grant_always(&change("src/main.rs"));
    policy.grant_always(&run("cargo test"));

    assert_eq!(policy.judge(&change("README.md")), Verdict::Allow);
    assert_eq!(policy.judge(&run("cargo test")), Verdict::Allow);
    assert_eq!(policy.judge(&run("cargo test --release")), Verdict::Ask);
}

#[test]
fn always_on_changes_never_covers_a_protected_file() {
    let mut policy = Policy::new(PermissionMode::Ask);
    policy.grant_always(&change("src/main.rs"));

    for path in [
        ".git/hooks/pre-commit",
        ".git/config",
        "vendor/lib/.git/hooks/post-checkout",
        ".claude/settings.json",
        ".cargo/config.toml",
        ".vscode/tasks.json",
        ".envrc",
        "tools/.envrc",
    ] {
        assert_eq!(policy.judge(&change(path)), Verdict::Ask, "{path}");
    }
}

#[test]
fn always_on_a_protected_file_covers_that_file_only() {
    let mut policy = Policy::new(PermissionMode::Ask);

    policy.grant_always(&change(".git/hooks/pre-commit"));

    assert_eq!(
        policy.judge(&change(".git/hooks/pre-commit")),
        Verdict::Allow
    );
    assert_eq!(policy.judge(&change(".git/hooks/pre-push")), Verdict::Ask);
    assert_eq!(policy.judge(&change("src/main.rs")), Verdict::Ask);
}

#[test]
fn granting_twice_keeps_one_grant() {
    let mut policy = Policy::new(PermissionMode::Ask);

    policy.grant_always(&run("ls"));
    policy.grant_always(&run("ls"));

    assert_eq!(policy.grants, [Grant::Command("ls".to_owned())]);
}

#[test]
fn protects_what_makes_code_run_and_nothing_that_only_looks_like_it() {
    assert!(is_protected(".git/hooks/pre-commit"));
    assert!(is_protected(
        "/Users/me/project/.claude/settings.local.json"
    ));
    assert!(is_protected(".envrc"));
    assert!(!is_protected("src/git/hooks.rs"));
    assert!(!is_protected(".gitignore"));
    assert!(!is_protected("docs/.envrc.example"));
    assert!(!is_protected("my.git/file"));
}
