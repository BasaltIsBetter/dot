use std::io::Write;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("dot-log-{name}-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    folder
}

fn bytes(count: u64) -> Vec<u8> {
    vec![b'x'; usize::try_from(count).unwrap()]
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
    writeln!(Log::open(&folder).unwrap(), "first").unwrap();

    writeln!(Log::open(&folder).unwrap(), "second").unwrap();

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
    let full = bytes(LONGEST_LOG + 1);
    fs::write(folder.join(LOG_FILE), &full).unwrap();

    writeln!(Log::open(&folder).unwrap(), "fresh").unwrap();

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
    fs::write(folder.join(LOG_FILE), bytes(LONGEST_LOG)).unwrap();

    drop(Log::open(&folder).unwrap());

    assert!(!folder.join(PREVIOUS_LOG_FILE).exists());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn starts_a_new_log_past_exactly_one_mebibyte() {
    assert_eq!(LONGEST_LOG, 1_048_576);
}

#[test]
fn starts_a_new_log_while_dot_runs_before_it_passes_one_mebibyte() {
    let folder = scratch("rotate-running");
    let mut log = Log::open(&folder).unwrap();
    let almost_full = bytes(LONGEST_LOG - 3);

    log.write_all(&almost_full).unwrap();
    log.write_all(b"next").unwrap();
    log.write_all(b"then").unwrap();

    assert_eq!(fs::read(folder.join(LOG_FILE)).unwrap(), b"nextthen");
    assert_eq!(
        fs::read(folder.join(PREVIOUS_LOG_FILE)).unwrap(),
        almost_full
    );
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn fills_the_log_to_exactly_one_mebibyte_while_dot_runs() {
    let folder = scratch("fill-running");
    let mut log = Log::open(&folder).unwrap();

    log.write_all(&bytes(LONGEST_LOG - 4)).unwrap();
    log.write_all(b"last").unwrap();

    assert_eq!(
        fs::metadata(folder.join(LOG_FILE)).unwrap().len(),
        LONGEST_LOG
    );
    assert!(!folder.join(PREVIOUS_LOG_FILE).exists());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn counts_what_the_log_held_when_it_was_opened() {
    let folder = scratch("count-held");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join(LOG_FILE), bytes(LONGEST_LOG - 3)).unwrap();
    let mut log = Log::open(&folder).unwrap();

    log.write_all(b"next").unwrap();

    assert_eq!(fs::read(folder.join(LOG_FILE)).unwrap(), b"next");
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn writes_whole_into_an_empty_log_a_line_longer_than_the_limit() {
    let folder = scratch("long-line");
    let mut log = Log::open(&folder).unwrap();
    let too_long = bytes(LONGEST_LOG + 10);

    log.write_all(&too_long).unwrap();

    assert_eq!(fs::read(folder.join(LOG_FILE)).unwrap(), too_long);
    assert!(!folder.join(PREVIOUS_LOG_FILE).exists());
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn keeps_writing_and_says_why_when_the_log_cannot_start_afresh() {
    let folder = scratch("cannot-rotate");
    fs::create_dir_all(folder.join(PREVIOUS_LOG_FILE).join("in-the-way")).unwrap();
    let mut log = Log::open(&folder).unwrap();
    log.write_all(&bytes(LONGEST_LOG)).unwrap();

    log.write_all(b"still logged\n").unwrap();
    log.write_all(b"and this\n").unwrap();

    let written = fs::read(folder.join(LOG_FILE)).unwrap();
    let after = String::from_utf8_lossy(&written[usize::try_from(LONGEST_LOG).unwrap()..]);
    assert!(
        after.starts_with("the log cannot start afresh: "),
        "{after}"
    );
    assert!(after.ends_with("\nstill logged\nand this\n"), "{after}");
    assert_eq!(after.matches("cannot start afresh").count(), 1);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn writes_what_is_logged_into_the_file() {
    let folder = scratch("written");
    let log = Log::open(&folder).unwrap();

    start_logging(log).unwrap();
    tracing::warn!("a reply failed");

    let written = fs::read_to_string(folder.join(LOG_FILE)).unwrap();
    assert!(written.contains("WARN"), "{written}");
    assert!(written.contains("a reply failed"), "{written}");
    assert!(!written.contains('\u{1b}'), "no colour codes: {written}");
    fs::remove_dir_all(folder).unwrap();
}
