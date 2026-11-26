use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use dot_api::{Detail, ToolContent, ToolEnd};

use super::*;

static NEXT_FOLDER: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct TempFolder(PathBuf);

impl TempFolder {
    pub(crate) fn new(name: &str) -> Self {
        let number = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("dot-tool-{name}-{}-{number}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn join(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    pub(crate) fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    pub(crate) fn pipe(&self, relative: &str) -> PathBuf {
        let path = self.join(relative);
        let made = std::process::Command::new("/usr/bin/mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(made.success());
        path
    }
}

pub(crate) const FAKE_LOGIN_PATH: &str = "/opt/dot-test/bin:/usr/bin:/bin";

pub(crate) fn fake_login_shell(folder: &TempFolder) -> LoginPath {
    let shell = folder.write(
        "login-shell",
        &format!("#!/bin/sh\nPATH={FAKE_LOGIN_PATH} exec /bin/sh -c \"$4\"\n"),
    );
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();
    LoginPath::new(Some(shell))
}

pub(crate) fn without_waiting<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, done) = std::sync::mpsc::channel();
    std::thread::spawn(move || sender.send(work()));
    done.recv_timeout(std::time::Duration::from_secs(5))
        .expect("the tool waited on something that never came")
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) struct Answering {
    decision: Decision,
    cancel_while_asking: bool,
    pub(crate) asked: RefCell<Vec<Permission>>,
}

impl Answering {
    pub(crate) fn with(decision: Decision) -> Self {
        Self {
            decision,
            cancel_while_asking: false,
            asked: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn actions(&self) -> Vec<Action> {
        self.asked
            .borrow()
            .iter()
            .map(|permission| permission.action.clone())
            .collect()
    }
}

impl Permissions for Answering {
    fn ask(&self, permission: &Permission, cancellation: &Cancellation) -> Decision {
        self.asked.borrow_mut().push(permission.clone());
        if self.cancel_while_asking {
            cancellation.cancel();
            return Decision::Deny;
        }
        self.decision
    }
}

fn run(toolbox: &mut LocalToolbox, call: &ToolCall, answering: &Answering) -> ToolOutput {
    toolbox.run(
        call,
        "toolu_1",
        answering,
        &Cancellation::new(),
        &mut |_| {},
    )
}

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn view(path: &str) -> ToolCall {
    ToolCall::View {
        path: path.to_owned(),
        lines: None,
    }
}

fn create(path: &str, text: &str) -> ToolCall {
    ToolCall::Create {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn reads_inside_the_folder_without_asking() {
    let folder = TempFolder::new("read-inside");
    folder.write("src/main.rs", "fn main() {}\n");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let answering = Answering::with(Decision::Deny);

    let output = run(&mut toolbox, &view("src/main.rs"), &answering);

    assert_eq!(text(&output), "     1\tfn main() {}\n");
    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(1)));
    assert_eq!(answering.actions().len(), 0);
}

#[test]
fn asks_before_reading_outside_the_folder_and_reads_nothing_when_refused() {
    let folder = TempFolder::new("read-outside");
    let elsewhere = TempFolder::new("read-outside-other");
    let secret = elsewhere.write("secret.txt", "hidden\n");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);
    let allowing = Answering::with(Decision::Allow);

    let refused = run(&mut toolbox, &view(secret.to_str().unwrap()), &refusing);
    let allowed = run(&mut toolbox, &view(secret.to_str().unwrap()), &allowing);

    assert_eq!(refused, ToolOutput::refused());
    assert_eq!(
        refusing.actions(),
        [Action::ReadOutside {
            path: secret.display().to_string()
        }]
    );
    assert_eq!(text(&allowed), "     1\thidden\n");
}

#[test]
fn a_link_that_leads_outside_reads_as_outside() {
    let folder = TempFolder::new("link-out");
    let elsewhere = TempFolder::new("link-out-other");
    let secret = elsewhere.write("secret.txt", "hidden\n");
    symlink(&secret, folder.join("innocent.txt")).unwrap();
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);

    let output = run(&mut toolbox, &view("innocent.txt"), &refusing);

    assert_eq!(output, ToolOutput::refused());
    assert_eq!(
        refusing.actions(),
        [Action::ReadOutside {
            path: secret.display().to_string()
        }]
    );
    assert_eq!(toolbox.inside("innocent.txt"), None);
}

#[test]
fn asks_before_creating_a_file_and_writes_nothing_when_refused() {
    let folder = TempFolder::new("create");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);
    let allowing = Answering::with(Decision::Allow);

    let refused = run(&mut toolbox, &create("src/new.rs", "one\ntwo\n"), &refusing);
    let missing_after_refusal = !folder.join("src").exists();
    let allowed = run(&mut toolbox, &create("src/new.rs", "one\ntwo\n"), &allowing);

    assert_eq!(refused, ToolOutput::refused());
    assert!(missing_after_refusal);
    assert_eq!(
        allowing.actions(),
        [Action::Change {
            path: "src/new.rs".to_owned()
        }]
    );
    assert_eq!(
        allowed.end,
        ToolEnd::Done(Detail::Changes {
            added: 2,
            removed: 0
        })
    );
    assert_eq!(
        fs::read_to_string(folder.join("src/new.rs")).unwrap(),
        "one\ntwo\n"
    );
}

#[test]
fn asks_about_a_change_outside_the_folder_with_its_whole_path() {
    let folder = TempFolder::new("change-outside");
    let elsewhere = TempFolder::new("change-outside-other");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);
    let target = elsewhere.join("x.txt");

    let output = run(
        &mut toolbox,
        &create(target.to_str().unwrap(), "x"),
        &refusing,
    );

    assert_eq!(output, ToolOutput::refused());
    assert_eq!(
        refusing.actions(),
        [Action::ChangeOutside {
            path: target.display().to_string()
        }]
    );
    assert!(!target.exists());
}

#[test]
fn asks_before_replacing_or_inserting() {
    let folder = TempFolder::new("edit-ask");
    let file = folder.write("a.txt", "one\ntwo\n");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);
    let replace = ToolCall::Replace {
        path: "a.txt".to_owned(),
        old: "one".to_owned(),
        new: "uno".to_owned(),
    };
    let insert = ToolCall::Insert {
        path: "a.txt".to_owned(),
        after_line: 1,
        text: "between".to_owned(),
    };

    let replaced = run(&mut toolbox, &replace, &refusing);
    let inserted = run(&mut toolbox, &insert, &refusing);

    assert_eq!(replaced, ToolOutput::refused());
    assert_eq!(inserted, ToolOutput::refused());
    assert_eq!(
        refusing.actions(),
        [
            Action::Change {
                path: "a.txt".to_owned()
            },
            Action::Change {
                path: "a.txt".to_owned()
            }
        ]
    );
    assert_eq!(fs::read_to_string(file).unwrap(), "one\ntwo\n");
}

#[test]
fn asks_before_every_command_and_runs_nothing_when_refused() {
    let folder = TempFolder::new("run-ask");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);
    let allowing = Answering::with(Decision::Allow);
    let command = ToolCall::Run {
        command: "touch made && echo done".to_owned(),
    };

    let refused = run(&mut toolbox, &command, &refusing);
    let made_after_refusal = folder.join("made").exists();
    let allowed = run(&mut toolbox, &command, &allowing);

    assert_eq!(refused, ToolOutput::refused());
    assert!(!made_after_refusal);
    assert_eq!(
        refusing.actions(),
        [Action::Run {
            command: "touch made && echo done".to_owned()
        }]
    );
    assert_eq!(text(&allowed), "done\n");
    assert!(folder.join("made").exists());
}

#[test]
fn runs_commands_with_the_path_of_the_login_shell() {
    let folder = TempFolder::new("toolbox-login-path");
    let mut toolbox =
        LocalToolbox::new(Some(folder.path().to_path_buf())).login_path(fake_login_shell(&folder));
    let command = ToolCall::Run {
        command: "printenv PATH".to_owned(),
    };

    let output = run(&mut toolbox, &command, &Answering::with(Decision::Allow));

    assert_eq!(text(&output), format!("{FAKE_LOGIN_PATH}\n"));
}

#[test]
fn restarts_the_shell_without_asking() {
    let folder = TempFolder::new("restart");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);

    let output = run(&mut toolbox, &ToolCall::RestartShell, &refusing);

    assert_eq!(output.end, ToolEnd::Done(Detail::Nothing));
    assert_eq!(refusing.actions().len(), 0);
}

#[test]
fn searches_inside_the_folder_without_asking_and_outside_after_asking() {
    let folder = TempFolder::new("search-ask");
    let elsewhere = TempFolder::new("search-ask-other");
    folder.write("a.rs", "needle\n");
    elsewhere.write("b.rs", "needle\n");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let refusing = Answering::with(Decision::Deny);
    let inside = ToolCall::SearchText(dot_api::TextSearch {
        pattern: "needle".to_owned(),
        folder: None,
        files: None,
        ignore_case: false,
        show: dot_api::SearchShow::Files,
    });
    let outside = ToolCall::FindFiles {
        pattern: "*.rs".to_owned(),
        folder: Some(elsewhere.path().display().to_string()),
    };

    let found_inside = run(&mut toolbox, &inside, &refusing);
    let found_outside = run(&mut toolbox, &outside, &refusing);

    assert_eq!(text(&found_inside), "a.rs\n");
    assert_eq!(found_outside, ToolOutput::refused());
    assert_eq!(
        refusing.actions(),
        [Action::ReadOutside {
            path: elsewhere.path().display().to_string()
        }]
    );
}

#[test]
fn without_a_folder_relative_paths_and_searches_fail() {
    let mut toolbox = LocalToolbox::new(None);
    let answering = Answering::with(Decision::Allow);
    let find = ToolCall::FindFiles {
        pattern: "*.rs".to_owned(),
        folder: None,
    };

    let viewed = run(&mut toolbox, &view("src/main.rs"), &answering);
    let found = run(&mut toolbox, &find, &answering);

    assert_eq!(viewed.end, ToolEnd::Failed);
    assert!(text(&viewed).contains("no folder"));
    assert_eq!(found.end, ToolEnd::Failed);
    assert_eq!(text(&found), NO_FOLDER_TO_SEARCH);
    assert_eq!(answering.actions().len(), 0);
    assert_eq!(toolbox.folder(), None);
    assert_eq!(toolbox.inside("/tmp"), None);
}

#[test]
fn a_cancelled_reply_runs_nothing_and_asks_nothing() {
    let folder = TempFolder::new("cancelled");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let answering = Answering::with(Decision::Allow);
    let cancellation = Cancellation::new();
    cancellation.cancel();

    let output = toolbox.run(
        &create("x.txt", "x"),
        "toolu_1",
        &answering,
        &cancellation,
        &mut |_| {},
    );

    assert_eq!(output, ToolOutput::stopped());
    assert_eq!(answering.actions().len(), 0);
    assert!(!folder.join("x.txt").exists());
}

#[test]
fn a_reply_stopped_while_asking_runs_nothing() {
    let folder = TempFolder::new("stopped-asking");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let answering = Answering {
        cancel_while_asking: true,
        ..Answering::with(Decision::Allow)
    };

    let output = run(&mut toolbox, &create("x.txt", "x"), &answering);

    assert_eq!(output, ToolOutput::stopped());
    assert!(!folder.join("x.txt").exists());
}

#[test]
fn hands_the_tool_id_to_the_question() {
    let folder = TempFolder::new("tool-id");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));
    let answering = Answering::with(Decision::Deny);

    toolbox.run(
        &create("x", "x"),
        "toolu_42",
        &answering,
        &Cancellation::new(),
        &mut |_| {},
    );

    assert_eq!(answering.asked.borrow()[0].tool_id, "toolu_42");
}

#[test]
fn finds_what_lies_inside_the_folder_even_before_it_exists() {
    let folder = TempFolder::new("holds");
    folder.write("src/lib.rs", "");
    let toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));

    assert!(toolbox.inside("src/lib.rs").is_some());
    assert!(toolbox.inside("src/not/yet.rs").is_some());
    assert_eq!(
        toolbox
            .inside(&folder.join("src").display().to_string())
            .as_deref(),
        Some("src")
    );
    assert!(toolbox.inside(".").is_some());
    assert_eq!(toolbox.inside("../elsewhere"), None);
    assert_eq!(toolbox.inside("/etc/hosts"), None);
    assert_eq!(toolbox.inside(""), None);
    assert_eq!(toolbox.folder(), Some(folder.path()));
}

#[test]
fn a_folder_that_does_not_exist_holds_nothing() {
    let toolbox = LocalToolbox::new(Some(PathBuf::from("/dot-tool-no-such-folder")));

    assert_eq!(toolbox.inside("/dot-tool-no-such-folder/a"), None);
}

#[test]
fn names_a_changed_file_relative_to_the_folder() {
    let folder = TempFolder::new("change-path");
    let mut toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));

    let output = run(
        &mut toolbox,
        &create("src/a.rs", "a\n"),
        &Answering::with(Decision::Allow),
    );

    assert_eq!(output.change.unwrap().path, "src/a.rs");
}

#[test]
fn tells_where_a_path_lies_once_links_and_case_are_followed() {
    let folder = TempFolder::new("inside-real");
    folder.write(".git/hooks/pre-commit", "");
    folder.write("src/lib.rs", "");
    std::os::unix::fs::symlink(folder.join(".git"), folder.join("innocent")).unwrap();
    let toolbox = LocalToolbox::new(Some(folder.path().to_path_buf()));

    assert_eq!(
        toolbox.inside("innocent/hooks/pre-commit").as_deref(),
        Some(".git/hooks/pre-commit")
    );
    if cfg!(target_os = "macos") {
        assert_eq!(toolbox.inside("SRC/LIB.RS").as_deref(), Some("src/lib.rs"));
    }
    assert_eq!(toolbox.inside("src/new.rs").as_deref(), Some("src/new.rs"));
    assert_eq!(toolbox.inside(".").as_deref(), Some("."));
}
