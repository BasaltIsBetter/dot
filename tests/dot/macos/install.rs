#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const INFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>dot</string>
<key>CFBundleIdentifier</key><string>dev.dot.install-test</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>
"#;

struct Trial {
    root: PathBuf,
}

impl Trial {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("dot-macos-install-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(root.join("a home")).unwrap();
        Self { root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("a home")
    }

    fn installed(&self) -> PathBuf {
        self.home().join("Applications/Dot.app")
    }

    fn launcher(&self) -> PathBuf {
        self.home().join(".local/bin/dot")
    }

    fn app(&self, says: &str) -> PathBuf {
        let app = self.root.join(format!("built-{says}/Dot.app"));
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/Info.plist"), INFO).unwrap();
        let program = app.join("Contents/MacOS/dot");
        fs::write(&program, format!("#!/bin/sh\necho {says} \"$@\"\n")).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let signed = Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(&app)
            .output()
            .unwrap();
        assert!(signed.status.success(), "{signed:?}");
        app
    }

    fn install(&self, app: &Path) -> Output {
        Command::new("bash")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("macos/install.sh"))
            .arg(app)
            .env("HOME", self.home())
            .env("TMPDIR", &self.root)
            .env("DOT_LAUNCH_SERVICES", "/usr/bin/true")
            .output()
            .unwrap()
    }
}

impl Drop for Trial {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

fn run(launcher: &Path) -> String {
    let output = Command::new(launcher).arg("my project").output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

fn says(app: &Path) -> String {
    fs::read_to_string(app.join("Contents/MacOS/dot")).unwrap()
}

#[test]
fn installs_the_app_and_a_launcher_that_runs_it() {
    let trial = Trial::new("fresh");
    let app = trial.app("first");

    let output = trial.install(&app);

    assert!(output.status.success(), "{output:?}");
    assert!(trial.installed().join("Contents/MacOS/dot").is_file());
    assert_eq!(run(&trial.launcher()), "first my project\n");
    let verified = Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(trial.installed())
        .status()
        .unwrap();
    assert!(verified.success());
}

#[test]
fn replaces_a_previous_install() {
    let trial = Trial::new("again");
    let first = trial.app("first");
    let second = trial.app("second");
    assert!(trial.install(&first).status.success());

    let output = trial.install(&second);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(run(&trial.launcher()), "second my project\n");
    let applications: Vec<_> = fs::read_dir(trial.home().join("Applications"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(applications, ["Dot.app"]);
}

#[test]
fn refuses_a_launcher_it_did_not_write() {
    let trial = Trial::new("foreign");
    let app = trial.app("first");
    fs::create_dir_all(trial.launcher().parent().unwrap()).unwrap();
    fs::write(trial.launcher(), "someone else's dot").unwrap();

    let output = trial.install(&app);

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(trial.launcher()).unwrap(),
        "someone else's dot"
    );
    assert!(!trial.installed().exists());
}

#[test]
fn keeps_the_installed_app_when_the_new_one_is_not_validly_signed() {
    let trial = Trial::new("tampered");
    let first = trial.app("first");
    let second = trial.app("second");
    assert!(trial.install(&first).status.success());
    fs::write(
        second.join("Contents/Info.plist"),
        INFO.replace("install-test", "changed"),
    )
    .unwrap();

    let output = trial.install(&second);

    assert!(!output.status.success());
    assert_eq!(says(&trial.installed()), "#!/bin/sh\necho first \"$@\"\n");
    assert_eq!(run(&trial.launcher()), "first my project\n");
}

#[test]
fn refuses_something_that_is_not_a_built_dot_app() {
    let trial = Trial::new("other");
    let app = trial.app("first");
    let renamed = app.with_file_name("Other.app");
    fs::rename(&app, &renamed).unwrap();

    let output = trial.install(&renamed);

    assert!(!output.status.success());
    assert!(!trial.installed().exists());
}
