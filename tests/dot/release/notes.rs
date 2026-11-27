use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CHANGELOG: &str = r"# Changelog de Dot

## [Unreleased]
- Pas encore sorti.

## [0.1.10] - 2026-12-20
- Dixième.

## [0.1.1] - 2026-12-01

### Nouveau
- **Les sessions reviennent** au lancement.

### Corrigé
- Le rideau ne saute plus.

## [0.1.0-beta]
- Bêta.

## [0.1.0]
- Première.
";

struct Trial {
    changelog: PathBuf,
}

impl Trial {
    fn new(name: &str, text: &str) -> Self {
        let changelog = std::env::temp_dir().join(format!(
            "dot-release-notes-{name}-{}.md",
            std::process::id()
        ));
        fs::write(&changelog, text).unwrap();
        Self { changelog }
    }

    fn notes(&self, version: &str) -> Output {
        notes_of(version, &self.changelog)
    }
}

impl Drop for Trial {
    fn drop(&mut self) {
        drop(fs::remove_file(&self.changelog));
    }
}

fn notes_of(version: &str, changelog: &Path) -> Output {
    Command::new("bash")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("release/notes.sh"))
        .arg(version)
        .arg(changelog)
        .output()
        .unwrap()
}

fn printed(output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn prints_the_section_of_a_version_with_its_subheadings() {
    let trial = Trial::new("section", CHANGELOG);

    let notes = printed(&trial.notes("0.1.1"));

    assert_eq!(
        notes,
        "### Nouveau\n- **Les sessions reviennent** au lancement.\n\n### Corrigé\n- Le rideau ne saute plus.\n"
    );
}

#[test]
fn prints_the_last_section_up_to_the_end_of_the_file() {
    let trial = Trial::new("last", CHANGELOG);

    assert_eq!(printed(&trial.notes("0.1.0")), "- Première.\n");
}

#[test]
fn tells_a_version_from_a_longer_one_that_starts_like_it() {
    let trial = Trial::new("prefix", CHANGELOG);

    assert_eq!(printed(&trial.notes("0.1.1")).lines().count(), 5);
    assert_eq!(printed(&trial.notes("0.1.10")), "- Dixième.\n");
    assert_eq!(printed(&trial.notes("0.1.0-beta")), "- Bêta.\n");
}

#[test]
fn refuses_a_version_without_a_section() {
    let trial = Trial::new("missing", CHANGELOG);

    let output = trial.notes("0.2.0");

    assert!(!output.status.success());
    assert_eq!(output.stdout, b"");
}

#[test]
fn refuses_a_section_without_notes() {
    let trial = Trial::new(
        "empty",
        "## [0.2.0] - 2026-12-24\n\n   \n## [0.1.0]\n- Première.\n",
    );

    let output = trial.notes("0.2.0");

    assert!(!output.status.success());
}

#[test]
fn refuses_a_changelog_that_does_not_exist() {
    let missing = std::env::temp_dir().join("dot-release-notes-nowhere.md");

    let output = notes_of("0.1.0", &missing);

    assert!(!output.status.success());
}
