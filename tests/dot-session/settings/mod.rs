use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use dot_api::{Column, Effort};

use super::*;
use crate::git::tests::ScratchFolder;
use crate::private::{BEING_WRITTEN, PRIVATE_FILE, PRIVATE_FOLDER};
use crate::settings::stored::tests::defaults;

#[test]
fn starts_from_the_defaults_without_a_file() {
    let folder = ScratchFolder::new("settings-none");

    assert_eq!(
        FilePreferences::new(Some(folder.path().to_path_buf())).load(&defaults()),
        defaults()
    );
}

#[test]
fn keeps_what_it_saved_for_the_next_launch() {
    let folder = ScratchFolder::new("settings-kept");
    let preferences = FilePreferences::new(Some(folder.path().to_path_buf()));
    let mut settings = defaults();
    settings.tuning.effort = Effort::Max;
    settings.column = Column::Narrow;

    preferences.save(&settings, &defaults()).unwrap();

    assert_eq!(preferences.load(&defaults()), settings);
    assert!(
        !folder
            .path()
            .join(format!("{FILE_NAME}{BEING_WRITTEN}"))
            .exists()
    );
}

#[test]
fn creates_its_folder_for_its_owner_only() {
    let parent = ScratchFolder::new("settings-folder");
    let folder = parent.path().join("Dot");
    let preferences = FilePreferences::new(Some(folder.clone()));

    preferences.save(&defaults(), &defaults()).unwrap();

    let folder_mode = fs::metadata(&folder).unwrap().permissions().mode() & 0o777;
    let file_mode = fs::metadata(folder.join(FILE_NAME))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(folder_mode, PRIVATE_FOLDER);
    assert_eq!(file_mode, PRIVATE_FILE);
}

#[test]
fn without_a_folder_keeps_nothing_and_says_so() {
    let preferences = FilePreferences::new(None);

    assert_eq!(preferences.load(&defaults()), defaults());
    assert_eq!(
        preferences.save(&defaults(), &defaults()),
        Err(PreferencesError(NO_FOLDER.to_owned()))
    );
}

#[test]
fn reads_no_more_than_its_largest_file() {
    let folder = ScratchFolder::new("settings-large");
    let padding = " ".repeat(usize::try_from(LARGEST_FILE).unwrap());
    folder.write(FILE_NAME, &format!("{padding}{{\"effort\":\"low\"}}"));

    let loaded = FilePreferences::new(Some(folder.path().to_path_buf())).load(&defaults());

    assert_eq!(loaded, defaults());
}

#[test]
fn reads_a_file_just_under_its_largest_size() {
    let folder = ScratchFolder::new("settings-fits");
    let setting = "{\"effort\":\"low\"}";
    let padding = " ".repeat(usize::try_from(LARGEST_FILE).unwrap() - setting.len());
    folder.write(FILE_NAME, &format!("{padding}{setting}"));

    let loaded = FilePreferences::new(Some(folder.path().to_path_buf())).load(&defaults());

    assert_eq!(loaded.tuning.effort, Effort::Low);
}

#[test]
fn a_folder_in_place_of_the_file_gives_the_defaults() {
    let folder = ScratchFolder::new("settings-dir");
    fs::create_dir(folder.path().join(FILE_NAME)).unwrap();

    let preferences = FilePreferences::new(Some(folder.path().to_path_buf()));

    assert_eq!(preferences.load(&defaults()), defaults());
}

#[test]
fn replaces_a_link_instead_of_writing_where_it_leads() {
    let folder = ScratchFolder::new("settings-link");
    let elsewhere = folder.write("elsewhere.json", "untouched");
    symlink(&elsewhere, folder.path().join(FILE_NAME)).unwrap();
    let mut settings = defaults();
    settings.tuning.effort = Effort::Low;

    FilePreferences::new(Some(folder.path().to_path_buf()))
        .save(&settings, &defaults())
        .unwrap();

    assert_eq!(fs::read_to_string(&elsewhere).unwrap(), "untouched");
    assert!(
        !fs::symlink_metadata(folder.path().join(FILE_NAME))
            .unwrap()
            .is_symlink()
    );
}

#[test]
fn says_why_it_could_not_save() {
    let folder = ScratchFolder::new("settings-blocked");
    let blocked = folder.write("not-a-folder", "");

    let saved = FilePreferences::new(Some(blocked)).save(&defaults(), &defaults());

    assert!(saved.is_err());
}

#[test]
fn reads_a_file_of_sixty_kibibytes() {
    let folder = ScratchFolder::new("settings-sixty");
    let padding = " ".repeat(60 * 1_024);
    folder.write(FILE_NAME, &format!("{padding}{{\"effort\":\"low\"}}"));

    let loaded = FilePreferences::new(Some(folder.path().to_path_buf())).load(&defaults());

    assert_eq!(loaded.tuning.effort, Effort::Low);
}
