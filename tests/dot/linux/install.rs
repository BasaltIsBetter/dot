use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PROGRAMS: [&str; 3] = ["dot", "dot-shell", "dot-agents"];
const DESKTOP_ENTRY: &str = "dev.dot.desktop.desktop";
const ICON: &str = "share/icons/hicolor/scalable/apps/dev.dot.desktop.svg";

struct Trial {
    root: PathBuf,
}

impl Trial {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("dot-linux-install-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(root.join("a home")).unwrap();
        Self { root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("a home")
    }

    fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    fn launcher(&self) -> PathBuf {
        self.home().join(".local/bin/dot")
    }

    fn bundle(&self, says: &str) -> PathBuf {
        let bundle = self.root.join(format!("bundle-{says}"));
        let crate_folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        fs::create_dir_all(bundle.join("bin")).unwrap();
        fs::create_dir_all(bundle.join("share/applications")).unwrap();
        fs::create_dir_all(bundle.join(ICON).parent().unwrap()).unwrap();
        for program in PROGRAMS {
            executable(
                &bundle.join("bin").join(program),
                &format!("#!/bin/sh\necho {says} \"$@\"\n"),
            );
        }
        fs::copy(
            crate_folder.join("linux").join(DESKTOP_ENTRY),
            bundle.join("share/applications").join(DESKTOP_ENTRY),
        )
        .unwrap();
        fs::copy(crate_folder.join("assets/icon.svg"), bundle.join(ICON)).unwrap();
        fs::copy(
            crate_folder.join("linux/install.sh"),
            bundle.join("install.sh"),
        )
        .unwrap();
        bundle
    }

    fn install(&self, bundle: &Path, data: Option<&Path>) -> Output {
        let mut command = Command::new("bash");
        command
            .arg(bundle.join("install.sh"))
            .env("HOME", self.home())
            .env_remove("XDG_DATA_HOME");
        if let Some(data) = data {
            command.env("XDG_DATA_HOME", data);
        }
        command.output().unwrap()
    }
}

impl Drop for Trial {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

fn executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn run(launcher: &Path) -> String {
    let output = Command::new(launcher).arg("project").output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn installs_the_programs_a_launcher_entry_and_the_icon() {
    let trial = Trial::new("fresh");
    let bundle = trial.bundle("first");

    let output = trial.install(&bundle, Some(&trial.data()));

    assert!(output.status.success(), "{output:?}");
    let home = trial.data().join("dot-app");
    assert_eq!(
        fs::read_link(trial.launcher()).unwrap(),
        home.join("bin/dot")
    );
    assert_eq!(run(&trial.launcher()), "first project\n");
    for program in PROGRAMS {
        assert!(home.join("bin").join(program).is_file(), "{program}");
    }
    let entry = fs::read_to_string(trial.data().join("applications").join(DESKTOP_ENTRY)).unwrap();
    assert!(entry.contains(&format!(
        "\nExec=\"{}\" %f\n",
        home.join("bin/dot").display()
    )));
    assert!(entry.contains("\nIcon=dev.dot.desktop\n"));
    assert!(entry.contains("\nStartupWMClass=dev.dot.desktop\n"));
    assert!(
        trial
            .data()
            .join("icons/hicolor/scalable/apps/dev.dot.desktop.svg")
            .is_file()
    );
}

#[test]
fn installs_under_the_home_share_folder_without_xdg_data_home() {
    let trial = Trial::new("default");
    let bundle = trial.bundle("first");

    let output = trial.install(&bundle, None);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_link(trial.launcher()).unwrap(),
        trial.home().join(".local/share/dot-app/bin/dot")
    );
}

#[test]
fn replaces_a_previous_install_and_leaves_nothing_beside_it() {
    let trial = Trial::new("again");
    let first = trial.bundle("first");
    let second = trial.bundle("second");
    assert!(trial.install(&first, Some(&trial.data())).status.success());

    let output = trial.install(&second, Some(&trial.data()));

    assert!(output.status.success(), "{output:?}");
    assert_eq!(run(&trial.launcher()), "second project\n");
    let mut left: Vec<_> = fs::read_dir(trial.data())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    left.sort();
    assert_eq!(left, ["applications", "dot-app", "icons"]);
}

#[test]
fn refuses_a_launcher_it_did_not_write() {
    let trial = Trial::new("foreign");
    let bundle = trial.bundle("first");
    fs::create_dir_all(trial.launcher().parent().unwrap()).unwrap();
    fs::write(trial.launcher(), "someone else's dot").unwrap();

    let output = trial.install(&bundle, Some(&trial.data()));

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(trial.launcher()).unwrap(),
        "someone else's dot"
    );
    assert!(!trial.data().join("dot-app").exists());
}

#[test]
fn refuses_a_bundle_without_one_of_its_programs() {
    let trial = Trial::new("incomplete");
    let bundle = trial.bundle("first");
    fs::remove_file(bundle.join("bin/dot-shell")).unwrap();

    let output = trial.install(&bundle, Some(&trial.data()));

    assert!(!output.status.success());
    assert!(!trial.data().join("dot-app").exists());
    assert!(!trial.launcher().exists());
}

#[test]
fn refuses_a_folder_a_launcher_entry_cannot_quote() {
    let trial = Trial::new("unquotable");
    let bundle = trial.bundle("first");
    let data = trial.root.join("da$ta");

    let output = trial.install(&bundle, Some(&data));

    assert!(!output.status.success());
    assert!(!data.join("dot-app").exists());
}

#[test]
fn leaves_the_sessions_and_settings_of_dot_alone() {
    let trial = Trial::new("data");
    let bundle = trial.bundle("first");
    let kept = trial.data().join("dot/sessions/index.json");
    fs::create_dir_all(kept.parent().unwrap()).unwrap();
    fs::write(&kept, "my sessions").unwrap();
    assert!(trial.install(&bundle, Some(&trial.data())).status.success());

    let output = trial.install(&bundle, Some(&trial.data()));

    assert!(output.status.success(), "{output:?}");
    assert_eq!(fs::read_to_string(kept).unwrap(), "my sessions");
}
