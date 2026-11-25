use super::*;
use crate::engine::tests::Scratch;

#[test]
fn an_agent_never_sees_a_whole_home_nor_a_whole_disk() {
    let home = Scratch::new("project-home");
    let project = home.path().join("project");
    fs::create_dir(&project).unwrap();

    for wide in [Path::new("/"), home.path(), home.path().parent().unwrap()] {
        let refused = Project::new(wide, Some(home.path()));
        assert!(matches!(refused, Err(ProjectError::TooWide(_))), "{wide:?}");
    }
    assert_eq!(
        Project::new(&project, Some(home.path())).unwrap().path(),
        project
    );
}

#[test]
fn a_link_to_the_home_is_the_home() {
    let home = Scratch::new("project-link");
    let elsewhere = Scratch::new("project-elsewhere");
    let link = elsewhere.path().join("link");
    std::os::unix::fs::symlink(home.path(), &link).unwrap();

    assert!(matches!(
        Project::new(&link, Some(home.path())),
        Err(ProjectError::TooWide(_))
    ));
}

#[test]
fn a_path_never_becomes_an_option_of_a_mount() {
    let scratch = Scratch::new("project-mount");
    for name in ["a,readonly=false", "a:b", "a\"b", "a\nb"] {
        let folder = scratch.path().join(name);
        fs::create_dir(&folder).unwrap();

        let refused = Project::new(&folder, None);

        assert!(
            matches!(refused, Err(ProjectError::Unmountable(_))),
            "{name:?}"
        );
    }
}

#[test]
fn a_file_is_no_project() {
    let scratch = Scratch::new("project-file");
    scratch.write("file", "");

    assert!(Project::new(&scratch.path().join("file"), None).is_err());
    assert!(Project::new(&scratch.path().join("nowhere"), None).is_err());
}
