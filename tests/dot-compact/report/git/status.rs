use super::*;

const LONG: &str = "On branch feat/mac-app\nYour branch is ahead of 'origin/feat/mac-app' by 2 commits.\n  (use \"git push\" to publish your local commits)\n\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n\tnew file:   crates/dot-compact/src/lib.rs\n\trenamed:    a.rs -> b.rs\n\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n\tmodified:   Cargo.toml\n\tdeleted:    old.rs\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\ttests/dot-compact/\n\nno changes added to commit (use \"git add\" and/or \"git commit -a\")\n";

#[test]
fn tells_a_status_without_its_advice() {
    let short = compact(&CommandLine::read("git status"), LONG);

    assert_eq!(
        short.as_deref(),
        Some(
            "On branch feat/mac-app\nYour branch is ahead of 'origin/feat/mac-app' by 2 commits.\nstaged:\nA crates/dot-compact/src/lib.rs\nR a.rs -> b.rs\nunstaged:\nM Cargo.toml\nD old.rs\nuntracked:\ntests/dot-compact/\n"
        )
    );
}

#[test]
fn keeps_the_lines_it_does_not_know() {
    let status = "HEAD detached at 305a7ed\nYou are currently rebasing branch 'x' on 'y'.\nUnmerged paths:\n\tboth modified:   src/lib.rs\n\tadded by us:     src/new.rs\nnothing to commit, working tree clean\n";

    assert_eq!(
        compact(&CommandLine::read("git -C repo status"), status).as_deref(),
        Some(
            "HEAD detached at 305a7ed\nYou are currently rebasing branch 'x' on 'y'.\nunmerged:\nUU src/lib.rs\nadded by us:     src/new.rs\nnothing to commit, working tree clean\n"
        )
    );
}

#[test]
fn leaves_alone_what_is_not_the_long_status_of_git() {
    assert_eq!(compact(&CommandLine::read("git log"), LONG), None);
    assert_eq!(compact(&CommandLine::read("echo status"), LONG), None);
    assert_eq!(
        compact(
            &CommandLine::read("git status --short"),
            " M Cargo.toml\n?? tests/\n"
        ),
        None
    );
    assert_eq!(
        compact(&CommandLine::read("git status"), "Sur la branche main\n"),
        None
    );
}

#[test]
fn leaves_as_it_is_what_another_command_printed_after_the_status() {
    let notes = "== notes ==\nStep 1\n  (optional) run the migration\n\n\tindented with a tab\nnothing to commit here\n";
    let both = format!("{LONG}{notes}");

    let short = compact(&CommandLine::read("git status && ./print-notes"), &both).unwrap();

    assert_eq!(
        short,
        format!(
            "On branch feat/mac-app\nYour branch is ahead of 'origin/feat/mac-app' by 2 commits.\nstaged:\nA crates/dot-compact/src/lib.rs\nR a.rs -> b.rs\nunstaged:\nM Cargo.toml\nD old.rs\nuntracked:\ntests/dot-compact/\n{notes}"
        )
    );
}

#[test]
fn knows_the_lines_of_a_status_from_those_of_another_command() {
    for line in [
        "On branch main",
        "HEAD detached at 305a7ed",
        "Not currently on any branch.",
        "Your branch is up to date with 'origin/main'.",
        "and have 1 and 2 different commits each, respectively.",
        "You have unmerged paths.",
        "You are currently rebasing branch 'x' on 'y'.",
        "All conflicts fixed but you are still merging.",
        "No commits yet",
        "Initial commit",
        "interactive rebase in progress; onto 305a7ed",
        "Last command done (1 command done):",
        "Next commands to do (2 remaining commands):",
        "No commands remaining.",
        "It took 2.31 seconds to enumerate untracked files.",
        "Changes to be committed:",
        "Untracked files:",
        "Ignored files:",
        "  (use \"git push\" to publish your local commits)",
        "\tmodified:   a.rs",
        "nothing to commit, working tree clean",
        "nothing added to commit but untracked files present (use \"git add\" to track)",
        "no changes added to commit (use \"git add\" and/or \"git commit -a\")",
        "",
        "  ",
    ] {
        assert!(is_of_the_status(line), "{line}");
    }
    for line in [
        "== notes ==",
        "Step 1",
        "   pick 305a7ed refactor",
        "commit 305a7ed",
        "Changes",
        " On branch main",
    ] {
        assert!(!is_of_the_status(line), "{line}");
    }
}
