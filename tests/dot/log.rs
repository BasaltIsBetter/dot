use std::io::Write;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("dot-log-{name}-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    folder
}

#[test]
fn keeps_the_log_in_the_logs_folder_of_the_user() {
    assert_eq!(
        log_folder(Path::new("/Users/me")),
        PathBuf::from("/Users/me/Library/Logs/Dot")
    );
}

#[test]
fn creates_the_folder_and_appends_to_the_log() {
    let folder = scratch("append");
    writeln!(open_log(&folder).unwrap(), "first").unwrap();

    writeln!(open_log(&folder).unwrap(), "second").unwrap();

    assert_eq!(
        fs::read_to_string(folder.join(LOG_FILE)).unwrap(),
        "first\nsecond\n"
    );
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn starts_a_new_log_past_one_mebibyte_and_keeps_the_previous_one() {
    let folder = scratch("rotate");
    fs::create_dir_all(&folder).unwrap();
    let full = vec![b'x'; usize::try_from(LONGEST_LOG).unwrap() + 1];
    fs::write(folder.join(LOG_FILE), &full).unwrap();

    writeln!(open_log(&folder).unwrap(), "fresh").unwrap();

    assert_eq!(
        fs::read_to_string(folder.join(LOG_FILE)).unwrap(),
        "fresh\n"
    );
    assert_eq!(
        fs::read(folder.join(PREVIOUS_LOG_FILE)).unwrap().len(),
        full.len()
    );
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn keeps_a_log_of_exactly_one_mebibyte() {
    let folder = scratch("limit");
    fs::create_dir_all(&folder).unwrap();
    let limit = vec![b'x'; usize::try_from(LONGEST_LOG).unwrap()];
    fs::write(folder.join(LOG_FILE), &limit).unwrap();

    drop(open_log(&folder).unwrap());

    assert!(!folder.join(PREVIOUS_LOG_FILE).exists());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn starts_a_new_log_past_exactly_one_mebibyte() {
    assert_eq!(LONGEST_LOG, 1_048_576);
}

#[test]
fn writes_what_is_logged_into_the_file() {
    let folder = scratch("written");
    let file = open_log(&folder).unwrap();

    start_logging(file).unwrap();
    tracing::warn!("a reply failed");

    let written = fs::read_to_string(folder.join(LOG_FILE)).unwrap();
    assert!(written.contains("WARN"), "{written}");
    assert!(written.contains("a reply failed"), "{written}");
    assert!(!written.contains('\u{1b}'), "no colour codes: {written}");
    fs::remove_dir_all(folder).unwrap();
}
