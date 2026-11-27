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
        (ReplyFailure::Oversized, Text::ReplyOversized),
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

#[test]
fn an_error_has_consumed_nothing_until_told_otherwise() {
    assert_eq!(
        ReplyError::new(ReplyFailure::Stopped).spent(),
        Reply::default()
    );
    assert_eq!(
        ReplyError::caused_by(ReplyFailure::Overloaded, io::Error::other(CAUSE)).spent(),
        Reply::default()
    );
}

#[test]
fn an_error_keeps_what_the_reply_consumed_before_it_failed() {
    let spent = Reply {
        context_tokens: 12_000,
        base_tokens: None,
        output_tokens: 900,
        cost: Cost::from_micro_dollars(48_000),
        limits: Limits {
            five_hours: Some(3),
            seven_days: None,
        },
    };

    let error = ReplyError::new(ReplyFailure::Stopped).with_spent(spent);

    assert_eq!(error.spent(), spent);
    assert_eq!(error.failure(), ReplyFailure::Stopped);
}

struct Forgetful;

impl Chat for Forgetful {
    fn reply(
        &mut self,
        _prompt: Prompt,
        _credential: &Credential,
        _cancellation: &Cancellation,
        _permissions: &dyn Permissions,
        _on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        Ok(Reply::default())
    }

    fn reset(&mut self) {}

    fn tune(&mut self, _tuning: Tuning) {}
}

#[test]
fn a_chat_that_keeps_no_memory_writes_nothing() {
    let mut memory = Vec::new();

    Forgetful
        .remember(&mut memory)
        .expect("a vector takes every byte");
    Forgetful.recall(b"anything".to_vec());

    assert_eq!(memory, Vec::<u8>::new());
}

#[test]
fn keeps_what_a_reply_spent_under_stable_names() {
    let reply = Reply {
        context_tokens: 1,
        base_tokens: Some(2),
        output_tokens: 3,
        cost: Cost::from_micro_dollars(4),
        limits: Limits {
            five_hours: Some(5),
            seven_days: None,
        },
    };
    let written = r#"{"context-tokens":1,"base-tokens":2,"output-tokens":3,"cost":4,"limits":{"five-hours":5,"seven-days":null}}"#;

    assert_eq!(
        serde_json::to_string(&reply).expect("a reply serializes"),
        written
    );
    assert_eq!(
        serde_json::from_str::<Reply>(written).expect("it reads back"),
        reply
    );
}

#[test]
fn reads_a_reply_with_missing_parts_as_nothing_spent_there() {
    let read: Reply = serde_json::from_str(r#"{"cost":7}"#).expect("missing parts default");

    assert_eq!(
        read,
        Reply {
            cost: Cost::from_micro_dollars(7),
            ..Reply::default()
        }
    );
}
