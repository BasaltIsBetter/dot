use std::io;

use super::*;

const CAUSE: &str = "connection reset";

#[test]
fn explains_each_failure_with_its_own_text() {
    let failures = [
        (ConnectFailure::KeyRejected, Text::SignInKeyRejected),
        (
            ConnectFailure::AnthropicUnreachable,
            Text::SignInAnthropicUnreachable,
        ),
        (
            ConnectFailure::ClaudeCodeMissing,
            Text::SignInClaudeCodeMissing,
        ),
        (
            ConnectFailure::ClaudeCodeFailed,
            Text::SignInClaudeCodeFailed,
        ),
        (ConnectFailure::AntMissing, Text::SignInAntMissing),
        (ConnectFailure::AccountFailed, Text::SignInAccountFailed),
        (ConnectFailure::NotConnected, Text::SignInNotConnected),
        (ConnectFailure::StorageFailed, Text::SignInStorageFailed),
    ];

    for (failure, text) in failures {
        assert_eq!(failure.text(), text);
    }
}

#[test]
fn displays_the_id_of_its_text() {
    let error = ConnectError::new(ConnectFailure::KeyRejected);

    assert_eq!(error.to_string(), "sign-in-key-rejected");
    assert_eq!(error.failure(), ConnectFailure::KeyRejected);
}

#[test]
fn keeps_its_technical_cause_as_source() {
    let cause = io::Error::other(CAUSE);

    let error = ConnectError::caused_by(ConnectFailure::AnthropicUnreachable, cause);

    assert_eq!(error.source().map(ToString::to_string), Some(CAUSE.into()));
}

#[test]
fn has_no_source_without_cause() {
    assert!(
        ConnectError::new(ConnectFailure::StorageFailed)
            .source()
            .is_none()
    );
}
