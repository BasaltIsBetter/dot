use std::path::Path;

use proptest::prelude::*;

use super::*;

fn arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn home() -> PathBuf {
    PathBuf::from("/Users/me")
}

#[test]
fn opens_the_folder_given_as_an_absolute_argument() {
    let folder = launch_folder(arguments(&["/Users/me/Desktop/Dot"]), Some(home()));

    assert_eq!(folder, Some(PathBuf::from("/Users/me/Desktop/Dot")));
}

#[test]
fn takes_a_relative_argument_from_the_current_folder() {
    let folder = launch_folder(arguments(&["Desktop/Dot"]), Some(home()));

    assert_eq!(folder, Some(PathBuf::from("/Users/me/Desktop/Dot")));
}

#[test]
fn skips_the_options_macos_adds() {
    let folder = launch_folder(
        arguments(&["-psn_0_42", "-NSDocumentRevisionsDebugMode", "Dot"]),
        Some(home()),
    );

    assert_eq!(folder, Some(PathBuf::from("/Users/me/Dot")));
}

#[test]
fn opens_the_current_folder_without_argument() {
    assert_eq!(launch_folder(arguments(&[]), Some(home())), Some(home()));
    assert_eq!(
        launch_folder(arguments(&["--exit"]), Some(home())),
        Some(home())
    );
}

#[test]
fn the_root_of_the_disk_is_no_project() {
    let finder_launch = launch_folder(arguments(&[]), Some(PathBuf::from("/")));
    let root_argument = launch_folder(arguments(&["/"]), Some(home()));

    assert_eq!(finder_launch, None);
    assert_eq!(root_argument, None);
}

#[test]
fn a_relative_argument_needs_a_current_folder() {
    assert_eq!(launch_folder(arguments(&["Dot"]), None), None);
}

#[test]
fn an_absolute_argument_needs_no_current_folder() {
    let folder = launch_folder(arguments(&["/Users/me/Dot"]), None);

    assert_eq!(folder, Some(PathBuf::from("/Users/me/Dot")));
}

#[test]
fn without_argument_nor_current_folder_there_is_no_folder() {
    assert_eq!(launch_folder(arguments(&[]), None), None);
}

proptest! {
    #[test]
    fn always_yields_an_absolute_folder_below_the_root(
        values in proptest::collection::vec("[-a-zA-Z0-9_./ ]{0,12}", 0..4),
        current in "(/[a-z]{1,8}){0,3}",
    ) {
        let current_folder = (!current.is_empty()).then(|| PathBuf::from(&current));
        let folder = launch_folder(values.iter().map(OsString::from), current_folder);

        if let Some(folder) = folder {
            prop_assert!(folder.is_absolute());
            prop_assert_ne!(folder.as_path(), Path::new("/"));
        }
    }
}
