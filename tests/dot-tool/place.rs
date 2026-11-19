use std::os::unix::fs::symlink;

use proptest::prelude::*;

use super::*;
use crate::toolbox::tests::TempFolder;

#[test]
fn joins_a_relative_path_to_the_folder() {
    let folder = TempFolder::new("place-relative");
    folder.write("src/main.rs", "");

    let place = locate(Some(folder.path()), "src/main.rs").unwrap();

    assert_eq!(place.path, folder.join("src/main.rs"));
    assert!(place.inside);
}

#[test]
fn keeps_an_absolute_path_and_judges_it_by_where_it_lies() {
    let folder = TempFolder::new("place-absolute");
    let elsewhere = TempFolder::new("place-absolute-other");
    let file = elsewhere.write("a.txt", "");

    let place = locate(Some(folder.path()), file.to_str().unwrap()).unwrap();

    assert_eq!(place.path, file);
    assert!(!place.inside);
}

#[test]
fn a_path_that_does_not_exist_yet_is_judged_by_the_folder_above_it() {
    let folder = TempFolder::new("place-missing");

    let inside = locate(Some(folder.path()), "new/deep/file.rs").unwrap();
    let dotted = locate(Some(folder.path()), "./new/./file.rs").unwrap();

    assert_eq!(inside.path, folder.join("new/deep/file.rs"));
    assert!(inside.inside);
    assert_eq!(dotted.path, folder.join("new/file.rs"));
}

#[test]
fn going_up_through_existing_folders_leaves_the_folder() {
    let folder = TempFolder::new("place-up");
    folder.write("src/a.rs", "");

    let back_inside = locate(Some(folder.path()), "src/../src/a.rs").unwrap();
    let out = locate(Some(folder.path()), "../escaped.txt").unwrap();

    assert_eq!(back_inside.path, folder.join("src/a.rs"));
    assert!(back_inside.inside);
    assert_eq!(
        out.path,
        folder.path().parent().unwrap().join("escaped.txt")
    );
    assert!(!out.inside);
}

#[test]
fn refuses_to_go_up_below_a_folder_that_does_not_exist() {
    let folder = TempFolder::new("place-up-missing");

    let resolved = resolve(Some(folder.path()), "ghost/../../escaped.txt");

    assert_eq!(resolved, Err(PlaceError::UpPastExisting));
}

#[test]
fn follows_links_to_where_they_lead() {
    let folder = TempFolder::new("place-link");
    let elsewhere = TempFolder::new("place-link-other");
    symlink(elsewhere.path(), folder.join("out")).unwrap();
    symlink(folder.join("real"), folder.join("in")).unwrap();
    folder.write("real/a.txt", "");

    let out = locate(Some(folder.path()), "out/new.txt").unwrap();
    let inside = locate(Some(folder.path()), "in/a.txt").unwrap();

    assert_eq!(out.path, elsewhere.join("new.txt"));
    assert!(!out.inside);
    assert_eq!(inside.path, folder.join("real/a.txt"));
    assert!(inside.inside);
}

#[test]
fn refuses_a_broken_link_rather_than_writing_through_it() {
    let folder = TempFolder::new("place-broken");
    let elsewhere = TempFolder::new("place-broken-other");
    symlink(elsewhere.join("not-yet.txt"), folder.join("trap.txt")).unwrap();

    let resolved = resolve(Some(folder.path()), "trap.txt");

    assert_eq!(
        resolved,
        Err(PlaceError::Unresolvable(
            folder.join("trap.txt").display().to_string()
        ))
    );
}

#[test]
fn refuses_empty_paths_nul_characters_and_relative_paths_without_a_folder() {
    let folder = TempFolder::new("place-refused");

    assert_eq!(resolve(Some(folder.path()), ""), Err(PlaceError::Empty));
    assert_eq!(resolve(Some(folder.path()), "a\0b"), Err(PlaceError::Nul));
    assert_eq!(resolve(None, "src/main.rs"), Err(PlaceError::NoFolder));
    assert!(resolve(None, "/tmp").is_ok());
    assert!(!locate(None, "/tmp").unwrap().inside);
}

#[test]
fn explains_each_refusal_to_claude() {
    let messages = [
        PlaceError::Empty,
        PlaceError::Nul,
        PlaceError::NoFolder,
        PlaceError::UpPastExisting,
        PlaceError::Unresolvable("/x".to_owned()),
    ]
    .map(|error| error.to_string());

    assert_eq!(messages[0], "The path is empty.");
    assert!(messages[1].contains("NUL"));
    assert!(messages[2].contains("absolute path"));
    assert!(messages[3].contains(".."));
    assert!(messages[4].starts_with("/x exists"));
}

#[test]
fn shows_a_place_inside_relative_to_the_folder() {
    let folder = TempFolder::new("place-shown");
    let elsewhere = TempFolder::new("place-shown-other");

    let inside = locate(Some(folder.path()), "src/a.rs").unwrap();
    let itself = locate(Some(folder.path()), ".").unwrap();
    let outside = locate(Some(folder.path()), elsewhere.path().to_str().unwrap()).unwrap();

    assert_eq!(shown(&inside, Some(folder.path())), "src/a.rs");
    assert_eq!(shown(&itself, Some(folder.path())), ".");
    assert_eq!(
        shown(&outside, Some(folder.path())),
        elsewhere.path().display().to_string()
    );
    assert_eq!(shown(&inside, None), inside.path.display().to_string());
}

#[test]
fn a_sibling_whose_name_extends_the_folder_is_outside() {
    let folder = TempFolder::new("place-sibling");
    let sibling = format!("{}-twin/a.txt", folder.path().display());

    let place = locate(Some(folder.path()), &sibling).unwrap();

    assert!(!place.inside);
}

proptest! {
    #[test]
    fn never_panics_and_never_calls_inside_what_lies_outside(
        parts in prop::collection::vec(prop_oneof![
            Just(".."), Just("."), Just("src"), Just("missing"), Just("out"), Just("a.txt"), Just("")
        ], 0..6),
        absolute in any::<bool>(),
    ) {
        let folder = TempFolder::new("place-property");
        folder.write("src/a.txt", "");
        let _ = symlink("/", folder.join("out"));
        let raw = format!("{}{}", if absolute { "/" } else { "" }, parts.join("/"));

        if let Ok(place) = locate(Some(folder.path()), &raw) {
            prop_assert_eq!(place.inside, place.path.starts_with(folder.path()));
            prop_assert!(place.path.components().all(|component| component != Component::ParentDir));
            prop_assert!(place.path.is_absolute());
        }
    }

    #[test]
    fn any_text_resolves_or_is_refused_without_panicking(raw in "\\PC{0,40}") {
        let folder = TempFolder::new("place-any");
        let _ = locate(Some(folder.path()), &raw);
    }
}
