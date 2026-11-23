use super::*;

#[test]
fn tells_the_user_why_a_file_could_not_be_used() {
    assert_eq!(WorkspaceError::Outside.text(), Text::EditorOutside);
    assert_eq!(WorkspaceError::NotAFile.text(), Text::EditorNotAFile);
    assert_eq!(WorkspaceError::TooLarge.text(), Text::EditorTooLarge);
    assert_eq!(WorkspaceError::NotText.text(), Text::EditorNotText);
    assert_eq!(
        WorkspaceError::Failed("busy".to_owned()).text(),
        Text::EditorFailed
    );
}

#[test]
fn writes_its_text_and_the_system_s_words_in_the_journal() {
    assert_eq!(WorkspaceError::Outside.to_string(), "editor-outside");
    assert_eq!(
        WorkspaceError::Failed("Permission denied".to_owned()).to_string(),
        "editor-failed: Permission denied"
    );
    assert_eq!(LARGEST_EDITED, 1_048_576);
}
