use std::fs;
use std::path::PathBuf;

use super::*;

struct Home(PathBuf);

impl Home {
    fn with(name: &str, file: &str, written: &[u8]) -> Self {
        let folder =
            std::env::temp_dir().join(format!("dot-settings-{}-{name}", std::process::id()));
        fs::create_dir_all(folder.join(".claude")).unwrap();
        fs::write(folder.join(".claude").join(file), written).unwrap();
        Self(folder)
    }

    fn summaries(&self) -> Summaries {
        summaries_for(Summaries::WhenTheyPay, &[&self.0])
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn leaves_summaries_to_claude_when_the_user_chose_a_window_for_every_model_or_for_one() {
    let for_all = Home::with(
        "all",
        "settings.json",
        br#"{"model":"opus","autoCompactWindow":300000}"#,
    );
    let for_one = Home::with(
        "one",
        "settings.local.json",
        br#"{"modelSettings":[{"claude-opus-5-5":{"autoCompactWindow":"400k"}}]}"#,
    );

    assert_eq!(for_all.summaries(), Summaries::LeftToClaude);
    assert_eq!(for_one.summaries(), Summaries::LeftToClaude);
}

#[test]
fn keeps_what_dot_wants_where_no_window_is_set() {
    let other = Home::with(
        "other",
        "settings.json",
        br#"{"model":"opus","note":"autoCompactWindow"}"#,
    );
    let not_json = Home::with("broken", "settings.json", b"autoCompactWindow: 300000");
    let elsewhere = Home::with(
        "elsewhere",
        "notes.json",
        br#"{"autoCompactWindow":300000}"#,
    );
    let nowhere = other.0.join("missing");

    assert_eq!(other.summaries(), Summaries::WhenTheyPay);
    assert_eq!(not_json.summaries(), Summaries::WhenTheyPay);
    assert_eq!(elsewhere.summaries(), Summaries::WhenTheyPay);
    assert_eq!(
        summaries_for(Summaries::WhenTheyPay, &[&nowhere]),
        Summaries::WhenTheyPay
    );
    assert_eq!(
        summaries_for(Summaries::WhenTheyPay, &[]),
        Summaries::WhenTheyPay
    );
    assert_eq!(
        summaries_for(Summaries::LeftToClaude, &[&nowhere]),
        Summaries::LeftToClaude
    );
}

#[test]
fn one_folder_that_sets_it_is_enough() {
    let without = Home::with("without", "settings.json", br#"{"model":"opus"}"#);
    let with = Home::with("with", "settings.json", br#"{"autoCompactWindow":300000}"#);

    assert_eq!(
        summaries_for(Summaries::WhenTheyPay, &[&without.0, &with.0]),
        Summaries::LeftToClaude
    );
}

#[test]
fn reads_a_long_file_as_long_as_it_stays_under_a_mebibyte() {
    let mut long = b"{".to_vec();
    long.extend_from_slice(&vec![b' '; 1_000_000]);
    long.extend_from_slice(br#""autoCompactWindow":300000}"#);
    let padded = Home::with("long", "settings.json", &long);

    assert_eq!(padded.summaries(), Summaries::LeftToClaude);
}

#[test]
fn reads_no_more_than_a_mebibyte_of_a_file() {
    let mut long = vec![b' '; 1_024 * 1_024];
    long.extend_from_slice(br#"{"autoCompactWindow":300000}"#);
    let padded = Home::with("padded", "settings.json", &long);

    assert_eq!(padded.summaries(), Summaries::WhenTheyPay);
}

#[test]
fn reads_no_setting_from_what_is_not_a_file_even_through_a_link() {
    let folder = std::env::temp_dir().join(format!("dot-settings-device-{}", std::process::id()));
    drop(fs::remove_dir_all(&folder));
    fs::create_dir_all(&folder).unwrap();
    let linked = folder.join("settings.json");
    std::os::unix::fs::symlink("/dev/zero", &linked).unwrap();
    let real = folder.join("real.json");
    fs::write(&real, r#"{"autoCompactWindow":100000}"#).unwrap();
    let to_real = folder.join("linked.json");
    std::os::unix::fs::symlink(&real, &to_real).unwrap();

    assert!(!sets_the_window(&linked));
    assert!(sets_the_window(&to_real));
    fs::remove_dir_all(&folder).unwrap();
}
