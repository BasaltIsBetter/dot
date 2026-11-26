use std::path::PathBuf;
use std::process::Command;

use proptest::prelude::*;
use serde_json::Value;

use super::*;

const RULES: &str = "# Rules\n\nEvery answer ends with `-- checked`.\n";

struct Project(PathBuf);

impl Project {
    fn named(name: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("dot-instructions-{}-{name}", std::process::id()));
        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        fs::create_dir_all(&folder).unwrap();
        Self(folder)
    }

    fn with(name: &str, written: &[u8]) -> Self {
        let project = Self::named(name);
        fs::write(project.file(), written).unwrap();
        project
    }

    fn file(&self) -> PathBuf {
        self.0.join("CLAUDE.md")
    }

    fn handed(&self) -> Option<Handed> {
        handed(&self.0)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn hook_command(handed: &Handed) -> String {
    let settings: Value = serde_json::from_str(&handed.settings).unwrap();
    let hook = &settings["hooks"]["SubagentStart"][0]["hooks"][0];
    assert_eq!(hook["type"], "command");
    hook["command"].as_str().unwrap().to_owned()
}

fn printed_by(command: &str) -> Value {
    let printed = Command::new("/bin/sh")
        .args(["-c", command])
        .output()
        .unwrap();
    assert!(printed.status.success(), "{command}");
    serde_json::from_slice(&printed.stdout).unwrap()
}

#[test]
fn hands_the_file_over_word_for_word_under_its_path() {
    let project = Project::with("whole", RULES.as_bytes());
    let path = project.file();

    let handed = project.handed().unwrap();

    assert_eq!(
        handed.note,
        format!(
            "\n\nContents of {} (project instructions, checked into the codebase):\n\n{RULES}",
            path.display()
        )
    );
}

#[test]
fn keeps_claude_code_from_loading_that_one_file_again() {
    let project = Project::with("excluded", RULES.as_bytes());

    let handed = project.handed().unwrap();

    let settings: Value = serde_json::from_str(&handed.settings).unwrap();
    let resolved = fs::canonicalize(project.file()).unwrap();
    let mut excluded = vec![project.file()];
    if resolved != project.file() {
        excluded.push(resolved);
    }
    assert_eq!(settings["claudeMdExcludes"], serde_json::json!(excluded));
    assert_eq!(settings.as_object().unwrap().len(), 2);
}

#[test]
fn excludes_the_file_under_the_path_a_link_to_its_folder_leads_to() {
    let real = Project::with("real-folder", RULES.as_bytes());
    let through_a_link = std::env::temp_dir().join(format!(
        "dot-instructions-{}-linked-folder",
        std::process::id()
    ));
    std::os::unix::fs::symlink(&real.0, &through_a_link).unwrap();

    let handed = handed(&through_a_link);
    fs::remove_file(&through_a_link).unwrap();

    let settings: Value = serde_json::from_str(&handed.unwrap().settings).unwrap();
    assert_eq!(
        settings["claudeMdExcludes"],
        serde_json::json!([
            through_a_link.join("CLAUDE.md"),
            fs::canonicalize(real.file()).unwrap()
        ])
    );
}

#[test]
fn has_each_sub_agent_told_to_read_the_file() {
    let project = Project::with("told", RULES.as_bytes());

    let handed = project.handed().unwrap();

    let told = printed_by(&hook_command(&handed));
    assert_eq!(told["hookSpecificOutput"]["hookEventName"], "SubagentStart");
    let said = told["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(said.contains(project.file().to_str().unwrap()), "{said}");
    assert!(
        said.starts_with("Before anything else, read the whole file"),
        "{said}"
    );
}

#[test]
fn leaves_to_claude_code_a_file_that_is_missing_or_is_not_a_plain_file() {
    let missing = Project::named("missing");
    let a_folder = Project::named("folder");
    fs::create_dir(a_folder.file()).unwrap();
    let a_link = Project::named("link");
    fs::write(a_link.0.join("real.md"), RULES).unwrap();
    std::os::unix::fs::symlink(a_link.0.join("real.md"), a_link.file()).unwrap();

    assert_eq!(missing.handed(), None);
    assert_eq!(a_folder.handed(), None);
    assert_eq!(a_link.handed(), None);
}

#[test]
fn leaves_to_claude_code_a_file_too_large_to_hand_over_or_that_is_not_text() {
    let length = usize::try_from(LARGEST_INSTRUCTIONS).unwrap();
    let largest = Project::with("largest", &vec![b'a'; length]);
    let too_large = Project::with("too-large", &vec![b'a'; length + 1]);
    let not_text = Project::with("binary", &[b'a', 0xff, 0xfe]);

    assert!(largest.handed().is_some());
    assert_eq!(too_large.handed(), None);
    assert_eq!(not_text.handed(), None);
}

#[test]
fn leaves_to_claude_code_a_file_that_imports_another() {
    for importing in [
        "@docs/rules.md\n",
        "# Rules\n\nSee @~/.claude/shared.md for the rest.\n",
        "```\ncode\n```\n@RTK.md\n",
        "\t@a\n",
    ] {
        let project = Project::with("imports", importing.as_bytes());

        assert_eq!(project.handed(), None, "{importing}");
    }
}

#[test]
fn takes_an_at_sign_in_code_in_an_address_or_alone_for_text() {
    for text in [
        "Use `@Getter` and `@Setter`.\n",
        "```java\n@RequiredArgsConstructor\nclass A {}\n```\n",
        "~~~\n@Data\n~~~\n",
        "Write to someone@example.com.\n",
        "The marks @ and + of ls.\n",
        "Ends with an at sign @",
        "A `code span` then `@inside` it.\n",
    ] {
        let project = Project::with("at-signs", text.as_bytes());

        assert!(project.handed().is_some(), "{text}");
    }
}

#[test]
fn leaves_to_claude_code_a_file_whose_path_a_pattern_or_a_shell_would_misread() {
    for name in [
        "it's", "a*b", "a[b", "a]b", "a{b", "a}b", "a(b", "a)b", "a|b", "a?b", "a!b", "a\"b",
        "a\\b", "a\nb",
    ] {
        let project = Project::with(name, RULES.as_bytes());

        assert_eq!(project.handed(), None, "{name}");
    }
    let spaced = Project::with("Application Support", RULES.as_bytes());
    assert!(spaced.handed().is_some());
}

#[test]
fn hands_nothing_over_from_a_folder_it_was_not_given_whole() {
    assert_eq!(handed(Path::new("relative/folder")), None);
    assert_eq!(handed(Path::new("")), None);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn whatever_the_folder_is_called_the_shell_prints_what_agents_are_told(name in "[ -.0-~éà\n\t]{1,24}") {
        let project = Project::with(&format!("named-{name}"), RULES.as_bytes());

        if let Some(handed) = project.handed() {
            let told = printed_by(&hook_command(&handed));
            let said = told["hookSpecificOutput"]["additionalContext"].as_str().unwrap().to_owned();
            prop_assert!(said.contains(project.file().to_str().unwrap()));
        }
    }

    #[test]
    fn hands_over_any_text_that_imports_nothing_as_it_is(text in "[a-z #\n.`~-]{0,200}") {
        let project = Project::with("any-text", text.as_bytes());

        let handed = project.handed().unwrap();

        prop_assert!(handed.note.ends_with(&text));
    }
}

#[test]
fn hands_over_no_instructions_holding_a_nul_byte() {
    let folder = std::env::temp_dir().join(format!("dot-instructions-nul-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    fs::create_dir_all(&folder).unwrap();
    let file = folder.join("CLAUDE.md");
    fs::write(&file, "# Rules\nnever\0this").unwrap();

    assert_eq!(read(&file), None);
    fs::write(&file, "# Rules\nalways this").unwrap();
    assert_eq!(read(&file).as_deref(), Some("# Rules\nalways this"));
    fs::remove_dir_all(&folder).unwrap();
}
