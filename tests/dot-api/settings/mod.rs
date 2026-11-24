use super::*;

#[test]
fn says_why_the_settings_were_not_saved() {
    let error = PreferencesError("disk full".to_owned());

    assert_eq!(error.text(), Text::SettingsUnsaved);
    assert_eq!(error.to_string(), "settings-unsaved: disk full");
}
