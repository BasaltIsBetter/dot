use std::fs::{self, Permissions};
use std::os::unix::fs::{PermissionsExt, symlink};

use super::*;
use crate::toolbox::tests::TempFolder;

fn workspace(folder: &TempFolder) -> LocalWorkspace {
    LocalWorkspace::new(Some(folder.path().to_path_buf()))
}

#[test]
fn reads_a_file_of_the_folder_with_plain_line_breaks() {
    let folder = TempFolder::new("workspace-read");
    folder.write("src/a.rs", "one\r\ntwo\n");

    assert_eq!(
        workspace(&folder).read("src/a.rs"),
        Ok("one\ntwo\n".to_owned())
    );
}

#[test]
fn refuses_what_leads_out_of_the_folder() {
    let folder = TempFolder::new("workspace-outside");
    let elsewhere = TempFolder::new("workspace-elsewhere");
    elsewhere.write("secret.txt", "no");
    symlink(elsewhere.join("secret.txt"), folder.join("link.txt")).unwrap();
    let outside = elsewhere.join("secret.txt");
    let workspace = workspace(&folder);

    assert_eq!(workspace.read("../x"), Err(WorkspaceError::Outside));
    assert_eq!(workspace.read("link.txt"), Err(WorkspaceError::Outside));
    assert_eq!(
        workspace.write("link.txt", "yes"),
        Err(WorkspaceError::Outside)
    );
    assert_eq!(workspace.list(".."), Err(WorkspaceError::Outside));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "no");
}

#[test]
fn refuses_relative_paths_without_a_folder() {
    let workspace = LocalWorkspace::new(None);
    let missing = LocalWorkspace::new(Some(std::env::temp_dir().join("dot-workspace-missing-0")));

    assert_eq!(workspace.read("a.rs"), Err(WorkspaceError::Outside));
    assert_eq!(workspace.list(""), Err(WorkspaceError::Outside));
    assert_eq!(missing.write("a.rs", "x"), Err(WorkspaceError::Outside));
}

#[test]
fn reads_writes_and_lists_a_file_anywhere_by_its_absolute_path() {
    let folder = TempFolder::new("workspace-session");
    let elsewhere = TempFolder::new("workspace-anywhere");
    let script = elsewhere.write("tool/run.py", "print(1)\r\n");
    elsewhere.write("tool/lib.py", "");
    symlink(&script, elsewhere.join("link.py")).unwrap();
    let absolute = |path: &std::path::Path| path.to_str().unwrap().to_owned();
    let tool = absolute(&elsewhere.join("tool"));

    for workspace in [workspace(&folder), LocalWorkspace::new(None)] {
        assert_eq!(
            workspace.read(&absolute(&script)),
            Ok("print(1)\n".to_owned())
        );
        assert_eq!(
            workspace.read(&absolute(&elsewhere.join("link.py"))),
            Ok("print(1)\n".to_owned())
        );
        assert_eq!(
            workspace.list(&tool),
            Ok(vec!["lib.py".to_owned(), "run.py".to_owned()])
        );
        assert_eq!(workspace.read(&tool), Err(WorkspaceError::NotAFile));
    }
    workspace(&folder)
        .write(&absolute(&script), "print(2)\n")
        .unwrap();

    assert_eq!(fs::read_to_string(&script).unwrap(), "print(2)\r\n");
}

#[test]
fn says_why_an_absolute_path_does_not_resolve() {
    let elsewhere = TempFolder::new("workspace-unresolved");
    symlink(elsewhere.join("gone.txt"), elsewhere.join("broken.txt")).unwrap();
    let broken = elsewhere.join("broken.txt");
    let workspace = LocalWorkspace::new(None);

    for path in [broken.to_str().unwrap(), "/tmp/a\0b"] {
        assert!(matches!(
            workspace.read(path),
            Err(WorkspaceError::Failed(cause)) if !cause.is_empty()
        ));
    }
    assert!(matches!(
        workspace.write(broken.to_str().unwrap(), "x"),
        Err(WorkspaceError::Failed(_))
    ));
    assert!(fs::symlink_metadata(elsewhere.join("gone.txt")).is_err());
}

#[test]
fn reads_only_regular_text_files_a_person_can_touch_up() {
    let folder = TempFolder::new("workspace-kinds");
    fs::create_dir_all(folder.join("sub")).unwrap();
    fs::write(folder.join("binary.bin"), [0xff, 0xfe, 0x00]).unwrap();
    let largest = usize::try_from(LARGEST_EDITED).unwrap();
    fs::write(folder.join("large.txt"), "x".repeat(largest + 1)).unwrap();
    fs::write(folder.join("just.txt"), "x".repeat(largest)).unwrap();
    let workspace = workspace(&folder);

    assert_eq!(workspace.read("sub"), Err(WorkspaceError::NotAFile));
    assert_eq!(workspace.read("binary.bin"), Err(WorkspaceError::NotText));
    assert_eq!(workspace.read("large.txt"), Err(WorkspaceError::TooLarge));
    assert_eq!(
        workspace.read("just.txt").map(|text| text.len()),
        Ok(largest)
    );
    assert!(matches!(
        workspace.read("none.txt"),
        Err(WorkspaceError::Failed(_))
    ));
}

#[test]
fn writes_in_place_keeping_permissions_and_windows_line_ends() {
    let folder = TempFolder::new("workspace-write");
    let windows = folder.write("windows.txt", "a\r\nb\r\n");
    let script = folder.write("run.sh", "echo\n");
    fs::set_permissions(&script, Permissions::from_mode(0o750)).unwrap();
    let workspace = workspace(&folder);

    workspace.write("windows.txt", "a\nB\n").unwrap();
    workspace.write("run.sh", "echo hi\n").unwrap();
    workspace.write("new.txt", "fresh").unwrap();

    assert_eq!(fs::read_to_string(windows).unwrap(), "a\r\nB\r\n");
    assert_eq!(fs::read_to_string(&script).unwrap(), "echo hi\n");
    assert_eq!(
        fs::metadata(&script).unwrap().permissions().mode() & 0o777,
        0o750
    );
    assert_eq!(fs::read_to_string(folder.join("new.txt")).unwrap(), "fresh");
    assert_eq!(workspace.write("", "x"), Err(WorkspaceError::NotAFile));
    let leftovers = fs::read_dir(folder.path())
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".dot-")
        })
        .count();
    assert_eq!(leftovers, 0);
}

#[test]
fn lists_a_folder_by_name_whatever_the_case_folders_marked() {
    let folder = TempFolder::new("workspace-list");
    folder.write("b.rs", "");
    folder.write("A.rs", "");
    folder.write(".hidden", "");
    fs::create_dir_all(folder.join("src")).unwrap();
    folder.write("src/inner.rs", "");
    let workspace = workspace(&folder);

    assert_eq!(
        workspace.list(""),
        Ok(vec![
            "A.rs".to_owned(),
            "b.rs".to_owned(),
            "src/".to_owned()
        ])
    );
    assert_eq!(workspace.list("src"), Ok(vec!["inner.rs".to_owned()]));
    assert!(matches!(
        workspace.list("b.rs"),
        Err(WorkspaceError::Failed(_))
    ));
}
