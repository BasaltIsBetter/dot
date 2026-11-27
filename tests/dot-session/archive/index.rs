use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use proptest::prelude::*;

use super::*;

fn session(id: u64, name: Option<&str>, hue: Hue, folder: Option<&str>) -> KeptSession {
    KeptSession {
        id: KeptId::new(id),
        name: name.map(str::to_owned),
        hue,
        folder: folder.map(PathBuf::from),
    }
}

fn arrangement() -> Arrangement {
    Arrangement {
        sessions: vec![
            session(1, Some("refonte"), Hue::Green, Some("/Users/me/Dot")),
            session(2, None, Hue::Violet, None),
        ],
        shown: Some(KeptId::new(2)),
    }
}

fn read_text(text: &str) -> Option<Arrangement> {
    read(text.as_bytes())
}

#[test]
fn reads_back_what_it_wrote() {
    let written = written(&arrangement()).unwrap();

    assert_eq!(read(&written), Some(arrangement()));
}

#[test]
fn writes_stable_names() {
    let written = String::from_utf8(written(&arrangement()).unwrap()).unwrap();

    assert_eq!(
        written,
        concat!(
            r#"{"format":1,"shown":"0000000000000002","sessions":["#,
            r#"{"id":"0000000000000001","name":"refonte","color":"green","folder":"/Users/me/Dot"},"#,
            r#"{"id":"0000000000000002","color":"violet"}]}"#
        )
    );
}

#[test]
fn reads_nothing_of_another_format_or_of_what_is_not_json() {
    assert_eq!(read_text(r#"{"format":2,"sessions":[]}"#), None);
    assert_eq!(read_text(r#"{"sessions":[]}"#), None);
    assert_eq!(read_text("not json"), None);
    assert_eq!(read_text(""), None);
}

#[test]
fn an_empty_index_keeps_no_session() {
    assert_eq!(read_text(r#"{"format":1}"#), Some(Arrangement::default()));
}

#[test]
fn a_damaged_entry_costs_only_that_session() {
    let read = read_text(concat!(
        r#"{"format":1,"shown":"0000000000000009","sessions":["#,
        r#"{"id":"not-an-id"},"#,
        r#"7,"#,
        r#"{"color":"green"},"#,
        r#"{"id":"0000000000000003","name":"","color":"mauve","folder":"relative/path"},"#,
        r#"{"id":"0000000000000003","name":"twin"},"#,
        r#"{"id":"0000000000000004","name":"bad\u0007name","folder":"/tmp"}]}"#
    ))
    .unwrap();

    assert_eq!(
        read.sessions,
        [
            session(3, None, Hue::Violet, None),
            session(4, None, Hue::Violet, Some("/tmp")),
        ]
    );
    assert_eq!(read.shown, None);
}

#[test]
fn keeps_a_name_up_to_its_longest() {
    let longest = "é".repeat(LONGEST_NAME);
    let longer = "é".repeat(LONGEST_NAME + 1);
    let text = format!(
        r#"{{"format":1,"sessions":[{{"id":"0000000000000001","name":"{longest}"}},{{"id":"0000000000000002","name":"{longer}"}}]}}"#
    );

    let read = read_text(&text).unwrap();

    assert_eq!(read.sessions[0].name.as_deref(), Some(longest.as_str()));
    assert_eq!(read.sessions[1].name, None);
}

#[test]
fn shows_no_session_it_does_not_keep() {
    let mut arrangement = arrangement();
    arrangement.shown = Some(KeptId::new(7));

    let written = String::from_utf8(written(&arrangement).unwrap()).unwrap();

    assert!(!written.contains("shown"), "{written}");
}

#[test]
fn leaves_out_a_session_whose_folder_it_cannot_write() {
    let mut arrangement = arrangement();
    arrangement.sessions[1].folder = Some(PathBuf::from(OsStr::from_bytes(b"/tmp/\xff")));

    let read = read(&written(&arrangement).unwrap()).unwrap();

    assert_eq!(read.sessions, arrangement.sessions[..1]);
    assert_eq!(read.shown, None);
}

proptest! {
    #[test]
    fn reading_anything_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = read(&bytes);
    }

    #[test]
    fn reads_back_any_readable_session(
        id in any::<u64>(),
        name in "[a-zA-Z0-9 éà_-]{1,64}",
        folder in "/[a-zA-Z0-9 ._-]{0,40}",
    ) {
        let kept = Arrangement {
            sessions: vec![session(id, Some(&name), Hue::Cyan, Some(&folder))],
            shown: Some(KeptId::new(id)),
        };
        let name_is_kept = !name.trim().is_empty();

        let read = read(&written(&kept).unwrap()).unwrap();

        prop_assert_eq!(read.sessions[0].id, KeptId::new(id));
        prop_assert_eq!(read.sessions[0].name.is_some(), name_is_kept);
        prop_assert_eq!(read.sessions[0].folder.as_deref(), Some(Path::new(&folder)));
        prop_assert_eq!(read.shown, Some(KeptId::new(id)));
    }
}
