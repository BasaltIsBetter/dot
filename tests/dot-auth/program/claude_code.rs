use super::*;

#[test]
fn finds_an_installed_claude_code_on_disk() {
    let directory = std::env::temp_dir().join(format!("dot-locate-claude-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join(PROGRAM), b"").unwrap();

    let found = locate_claude_code(None, Some(directory.as_os_str()));

    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(found, Some(directory.join(PROGRAM)));
}

#[test]
fn a_program_that_succeeds_signs_in() {
    let cli = ClaudeCodeCli::new(PathBuf::from("/usr/bin/true"));

    assert!(cli.is_signed_in().unwrap());
    assert!(cli.sign_in(&Cancellation::new()).is_ok());
}

#[test]
fn a_program_that_fails_is_signed_out_and_fails_to_sign_in() {
    let cli = ClaudeCodeCli::new(PathBuf::from("/usr/bin/false"));

    assert!(!cli.is_signed_in().unwrap());
    assert_eq!(
        cli.sign_in(&Cancellation::new()).unwrap_err().failure(),
        ConnectFailure::ClaudeCodeFailed
    );
}

#[test]
fn a_missing_program_fails_to_sign_in() {
    let cli = ClaudeCodeCli::new(PathBuf::from("/nonexistent/claude"));

    assert_eq!(
        cli.is_signed_in().unwrap_err().failure(),
        ConnectFailure::ClaudeCodeFailed
    );
}
