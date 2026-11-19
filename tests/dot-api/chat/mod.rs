use std::io;

use super::*;

const CAUSE: &str = "HTTP status 529";

#[test]
fn explains_each_failure_with_its_own_text() {
    let failures = [
        (ReplyFailure::Stopped, Text::ReplyStopped),
        (ReplyFailure::Unauthorized, Text::ReplyUnauthorized),
        (ReplyFailure::Rejected, Text::ReplyRejected),
        (ReplyFailure::RateLimited, Text::ReplyRateLimited),
        (ReplyFailure::Overloaded, Text::ReplyOverloaded),
        (ReplyFailure::Unreachable, Text::ReplyUnreachable),
        (ReplyFailure::Declined, Text::ReplyDeclined),
        (ReplyFailure::ClaudeCodeFailed, Text::ReplyClaudeCodeFailed),
    ];

    for (failure, text) in failures {
        assert_eq!(failure.text(), text);
    }
}

#[test]
fn displays_the_id_of_its_text() {
    let error = ReplyError::new(ReplyFailure::Overloaded);

    assert_eq!(error.to_string(), "reply-overloaded");
    assert_eq!(error.failure(), ReplyFailure::Overloaded);
}

#[test]
fn keeps_its_technical_cause_as_source() {
    let error = ReplyError::caused_by(ReplyFailure::Overloaded, io::Error::other(CAUSE));

    assert_eq!(error.source().map(ToString::to_string), Some(CAUSE.into()));
}

#[test]
fn has_no_source_without_cause() {
    assert!(ReplyError::new(ReplyFailure::Stopped).source().is_none());
}

#[test]
fn a_reply_without_report_has_consumed_nothing() {
    let reply = Reply::default();

    assert_eq!(reply.context_tokens, 0);
    assert_eq!(reply.cost, Cost::default());
    assert_eq!(reply.limits, Limits::default());
}
