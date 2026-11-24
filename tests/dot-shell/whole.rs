use std::fs::File;
use std::os::unix::fs::{PermissionsExt, symlink};

use super::*;

const PERMISSIONS: u32 = 0o777;

fn folder(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("dot-shell-whole-{}-{name}", std::process::id()));
    fs::remove_dir_all(&folder).unwrap_or_default();
    folder
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn mode_of(path: &std::path::Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & PERMISSIONS
}

#[test]
fn keeps_an_output_in_a_file_only_its_owner_can_read() {
    let folder = folder("kept").join("nested");
    let kept = WholeOutputs::new(folder.clone(), at(1_795_000_000), 42);

    let path = kept.keep(b"the whole output\n\xff").unwrap();

    assert_eq!(path, folder.join("40000-42"));
    assert_eq!(fs::read(&path).unwrap(), b"the whole output\n\xff");
    assert_eq!(mode_of(&path), 0o600);
    assert_eq!(mode_of(&folder), 0o700);
    fs::remove_dir_all(folder.parent().unwrap()).unwrap();
}

#[test]
fn names_the_file_after_the_time_and_the_process_of_the_command() {
    let folder = folder("named");

    let first = WholeOutputs::new(folder.clone(), at(7), 1)
        .keep(b"a")
        .unwrap();
    let second = WholeOutputs::new(folder.clone(), at(7), 2)
        .keep(b"b")
        .unwrap();
    let before_the_first_second = UNIX_EPOCH.checked_sub(Duration::from_secs(5)).unwrap();
    let before_any_clock = WholeOutputs::new(folder.clone(), before_the_first_second, 3)
        .keep(b"c")
        .unwrap();

    assert_eq!(first, folder.join("7-1"));
    assert_eq!(second, folder.join("7-2"));
    assert_eq!(before_any_clock, folder.join("0-3"));
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn never_writes_over_an_output_already_kept() {
    let folder = folder("twice");
    let kept = WholeOutputs::new(folder.clone(), at(7), 1);

    let path = kept.keep(b"first").unwrap();
    let again = kept.keep(b"second");

    assert_eq!(again.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(path).unwrap(), b"first");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn refuses_a_folder_other_users_can_open_and_a_link_to_a_folder() {
    let open = folder("open");
    let target = folder("target");
    let link = folder("link");
    fs::create_dir_all(&open).unwrap();
    fs::set_permissions(&open, fs::Permissions::from_mode(0o750)).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&target, &link).unwrap();

    let in_the_open = WholeOutputs::new(open.clone(), at(7), 1).keep(b"secret");
    let through_the_link = WholeOutputs::new(link.clone(), at(7), 1).keep(b"secret");

    assert_eq!(
        in_the_open.unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        through_the_link.unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(fs::read_dir(&open).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    fs::remove_dir_all(&open).unwrap();
    fs::remove_file(&link).unwrap();
    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn says_when_the_folder_cannot_be_made() {
    let taken = folder("taken");
    File::create(&taken).unwrap();

    let failed = WholeOutputs::new(taken.clone(), at(7), 1).keep(b"x");

    assert!(failed.is_err());
    fs::remove_file(&taken).unwrap();
}

#[test]
fn forgets_the_outputs_kept_for_more_than_a_day_and_no_other() {
    let folder = folder("old");
    let now = SystemTime::now();
    let a_day = 24 * 60;
    WholeOutputs::new(folder.clone(), now, 0).keep(b"").unwrap();
    for (name, minutes_ago) in [
        ("day-old.txt", Some(a_day + 1)),
        ("a-day-exactly.txt", Some(a_day)),
        ("almost.txt", Some(a_day - 1)),
        ("recent.txt", Some(60)),
        ("from-the-future.txt", None),
    ] {
        let file = File::create(folder.join(name)).unwrap();
        let written = match minutes_ago {
            Some(minutes) => now.checked_sub(Duration::from_mins(minutes)).unwrap(),
            None => now.checked_add(Duration::from_hours(1)).unwrap(),
        };
        file.set_modified(written).unwrap();
    }

    WholeOutputs::new(folder.clone(), now, 1)
        .keep(b"new")
        .unwrap();
    let mut left: Vec<String> = fs::read_dir(&folder)
        .unwrap()
        .map(|file| file.unwrap().file_name().into_string().unwrap())
        .filter(|name| !name.ends_with("-0") && !name.ends_with("-1"))
        .collect();
    left.sort();

    assert_eq!(
        left,
        [
            "a-day-exactly.txt",
            "almost.txt",
            "from-the-future.txt",
            "recent.txt"
        ]
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn keeps_no_more_of_an_output_than_it_may_and_says_it_is_not_whole() {
    let folder = folder("capped");
    let mut whole_outputs = WholeOutputs::new(folder.clone(), at(7), 1);
    whole_outputs.most_kept = 10;
    let mut kept = whole_outputs.start().unwrap();

    kept.write(b"0123").unwrap();
    kept.write(b"456789").unwrap();
    let whole_at_the_limit = kept.is_whole();
    kept.write(b"ab").unwrap();
    kept.write(b"cd").unwrap();

    assert!(whole_at_the_limit);
    assert!(!kept.is_whole());
    assert_eq!(fs::read(kept.path()).unwrap(), b"0123456789");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn keeps_the_start_of_the_piece_that_goes_past_what_it_may_keep() {
    let folder = folder("capped-within-a-piece");
    let mut whole_outputs = WholeOutputs::new(folder.clone(), at(7), 1);
    whole_outputs.most_kept = 5;
    let mut kept = whole_outputs.start().unwrap();

    kept.write(b"0123456789").unwrap();

    assert!(!kept.is_whole());
    assert_eq!(fs::read(kept.path()).unwrap(), b"01234");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn keeps_64_mebibytes_of_an_output_at_most() {
    let folder = folder("default-cap");

    let whole_outputs = WholeOutputs::new(folder, at(7), 1);

    assert_eq!(whole_outputs.most_kept, 67_108_864);
    assert_eq!(MOST_MEBIBYTES_KEPT, 64);
}

#[test]
fn forgets_a_file_and_leaves_the_others() {
    let folder = folder("forgotten");
    let other = WholeOutputs::new(folder.clone(), at(7), 1)
        .keep(b"other")
        .unwrap();
    let kept = WholeOutputs::new(folder.clone(), at(7), 2).start().unwrap();
    let path = kept.path().to_path_buf();

    kept.forget().unwrap();

    assert_eq!(path, folder.join("7-2"));
    assert!(!path.exists());
    assert!(other.exists());
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn accepts_a_folder_of_the_user_who_writes_the_file_and_refuses_another_users() {
    let refused = of_one_owner(0, 501).unwrap_err();

    assert!(of_one_owner(501, 501).is_ok());
    assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(
        refused.to_string(),
        "the folder of whole outputs belongs to another user"
    );
    assert!(of_one_owner(501, 0).is_err());
}

fn open_to_others(name: &str) -> PathBuf {
    let open = folder(name);
    fs::create_dir_all(&open).unwrap();
    fs::set_permissions(&open, fs::Permissions::from_mode(0o750)).unwrap();
    open
}

#[test]
fn keeps_an_output_in_the_fallback_when_its_folder_is_open_to_others() {
    let open = open_to_others("open-first");
    let fallback = folder("fallback-of-open");

    let path = WholeOutputs::new(open.clone(), at(7), 1)
        .or_else_in(fallback.clone())
        .keep(b"secret")
        .unwrap();

    assert_eq!(path, fallback.join("7-1"));
    assert_eq!(fs::read(&path).unwrap(), b"secret");
    assert_eq!(mode_of(&fallback), 0o700);
    assert_eq!(fs::read_dir(&open).unwrap().count(), 0);
    fs::remove_dir_all(&open).unwrap();
    fs::remove_dir_all(&fallback).unwrap();
}

#[test]
fn keeps_an_output_in_the_fallback_when_its_folder_cannot_be_made() {
    let taken = folder("taken-first");
    File::create(&taken).unwrap();
    let fallback = folder("fallback-of-taken");

    let path = WholeOutputs::new(taken.clone(), at(7), 1)
        .or_else_in(fallback.clone())
        .keep(b"x")
        .unwrap();

    assert_eq!(path, fallback.join("7-1"));
    fs::remove_file(&taken).unwrap();
    fs::remove_dir_all(&fallback).unwrap();
}

#[test]
fn leaves_the_fallback_alone_while_its_folder_takes_the_output() {
    let first = folder("first-taken");
    let fallback = folder("fallback-unused");

    let path = WholeOutputs::new(first.clone(), at(7), 1)
        .or_else_in(fallback.clone())
        .keep(b"x")
        .unwrap();

    assert_eq!(path, first.join("7-1"));
    assert!(!fallback.exists());
    fs::remove_dir_all(&first).unwrap();
}

#[test]
fn refuses_an_output_when_the_fallback_is_open_to_others_too() {
    let open = open_to_others("open-both-first");
    let also_open = open_to_others("open-both-fallback");

    let refused = WholeOutputs::new(open.clone(), at(7), 1)
        .or_else_in(also_open.clone())
        .keep(b"secret");

    assert_eq!(refused.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(fs::read_dir(&open).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&also_open).unwrap().count(), 0);
    fs::remove_dir_all(&open).unwrap();
    fs::remove_dir_all(&also_open).unwrap();
}

#[test]
fn keeps_outputs_in_the_own_temporary_folder_only_when_it_is_named_by_an_absolute_path() {
    assert_eq!(
        in_own_temporary_folder(Some("/var/folders/ab/xyz/T/".into())),
        Some(PathBuf::from("/var/folders/ab/xyz/T/dot-whole"))
    );
    assert_eq!(in_own_temporary_folder(Some("tmp".into())), None);
    assert_eq!(in_own_temporary_folder(Some("".into())), None);
    assert_eq!(in_own_temporary_folder(None), None);
}

fn kept_files(folder: &Path) -> Vec<String> {
    let mut left: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|file| file.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    left
}

#[test]
fn forgets_the_oldest_outputs_until_a_new_one_fits_under_the_ceiling() {
    let folder = folder("ceiling");
    let now = SystemTime::now();
    fs::create_dir_all(&folder).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
    for (name, minutes_ago) in [("oldest", 30), ("older", 20), ("newer", 10), ("newest", 5)] {
        fs::write(folder.join(name), [b'x'; 100]).unwrap();
        let file = File::options().write(true).open(folder.join(name)).unwrap();
        file.set_modified(now.checked_sub(Duration::from_mins(minutes_ago)).unwrap())
            .unwrap();
    }
    let mut keeper = WholeOutputs::new(folder.clone(), now, 9);
    keeper.most_kept = 100;
    keeper.most_in_all = 300;

    keeper.keep(b"new").unwrap();

    let mut expected = vec![keeper_name(now, 9), "newer".to_owned(), "newest".to_owned()];
    expected.sort();
    assert_eq!(kept_files(&folder), expected);
    fs::remove_dir_all(folder).unwrap();
}

fn keeper_name(now: SystemTime, process: u32) -> String {
    let second = now.duration_since(UNIX_EPOCH).unwrap().as_secs() % (24 * 60 * 60);
    format!("{second}-{process}")
}

#[test]
fn forgets_nothing_while_the_outputs_and_a_new_one_fit_under_the_ceiling() {
    let folder = folder("under");
    let now = SystemTime::now();
    fs::create_dir_all(&folder).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["first", "second"] {
        fs::write(folder.join(name), [b'x'; 100]).unwrap();
    }
    let mut keeper = WholeOutputs::new(folder.clone(), now, 9);
    keeper.most_kept = 100;
    keeper.most_in_all = 300;

    keeper.keep(b"new").unwrap();

    assert_eq!(kept_files(&folder).len(), 3);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn keeps_half_a_gibibyte_of_outputs_at_most_by_default() {
    let keeper = WholeOutputs::new(PathBuf::from("/nowhere"), at(0), 0);

    assert_eq!(keeper.most_in_all, 512 * 1_024 * 1_024);
    assert_eq!(MOST_MEBIBYTES_IN_ALL, 512);
}
