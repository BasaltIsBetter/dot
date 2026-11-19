use super::*;

fn failure_of(result: Result<(), ConnectError>) -> dot_api::ConnectFailure {
    result.unwrap_err().failure()
}

#[test]
fn the_real_connector_asks_for_claude_code_when_it_is_missing() {
    let connector = AnthropicConnector::new(None, None, None);

    let failure = failure_of(connector.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, dot_api::ConnectFailure::ClaudeCodeMissing);
}

#[test]
fn the_real_connector_reports_a_claude_code_that_cannot_sign_in() {
    let connector = AnthropicConnector::new(Some(PathBuf::from("/usr/bin/false")), None, None);

    let failure = failure_of(connector.connect_with_claude_code(&Cancellation::new()));

    assert_eq!(failure, dot_api::ConnectFailure::ClaudeCodeFailed);
}

#[test]
fn the_real_connector_asks_for_ant_when_it_is_missing() {
    let connector = AnthropicConnector::new(None, None, None);

    let failure = failure_of(connector.connect_with_anthropic_account(&Cancellation::new()));

    assert_eq!(failure, dot_api::ConnectFailure::AntMissing);
}

#[test]
fn the_real_connector_reports_an_ant_that_cannot_sign_in() {
    let connector = AnthropicConnector::new(None, Some(PathBuf::from("/usr/bin/false")), None);

    let failure = failure_of(connector.connect_with_anthropic_account(&Cancellation::new()));

    assert_eq!(failure, dot_api::ConnectFailure::AccountFailed);
}
