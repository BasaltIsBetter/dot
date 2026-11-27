use dot_api::{Effort, Model};
use serde_json::{Value, json};

use super::*;
use crate::context::Request;

const SESSION_ID: &str = "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0";

fn fresh() -> ClaudeCodeChat {
    ClaudeCodeChat::new(Model::OPUS_5_5, Effort::High, None, None, None)
}

fn kept_of(chat: &ClaudeCodeChat) -> Value {
    serde_json::to_value(chat.kept()).unwrap()
}

fn grown_ledger() -> Ledger {
    let mut ledger = Ledger::NOTHING_YET;
    ledger.request(Request {
        read: 50_000,
        written: 1_000,
        written_for_an_hour: 200,
    });
    ledger
}

#[test]
fn takes_up_the_session_what_it_billed_and_its_ledger() {
    let mut before = fresh();
    before.session = SessionId::parse(SESSION_ID);
    before.billed = 1_234;
    before.ledger = grown_ledger();
    let mut after = fresh();

    after.take_up(&kept_of(&before));

    assert_eq!(after.session, before.session);
    assert_eq!(after.billed, 1_234);
    assert_eq!(after.ledger, grown_ledger());
    assert_ne!(after.ledger, Ledger::NOTHING_YET);
}

#[test]
fn keeps_no_session_until_claude_code_gave_one() {
    let kept = kept_of(&fresh());
    let mut after = fresh();

    after.take_up(&kept);

    assert_eq!(kept.get("session"), None);
    assert_eq!(after.session, None);
}

#[test]
fn takes_nothing_up_without_a_session_claude_code_would_take() {
    for session in [
        json!("not-a-session"),
        json!(7),
        json!(null),
        json!("--resume"),
    ] {
        let mut after = fresh();

        after.take_up(&json!({"session": session, "billed": 9}));

        assert_eq!(after.session, None, "{session}");
        assert_eq!(after.billed, 0, "{session}");
    }
}

#[test]
fn a_damaged_bill_or_ledger_costs_only_itself() {
    let mut after = fresh();

    after.take_up(&json!({"session": SESSION_ID, "billed": "lots", "ledger": {"base": -1}}));

    assert_eq!(after.session, SessionId::parse(SESSION_ID));
    assert_eq!(after.billed, 0);
    assert_eq!(after.ledger, Ledger::NOTHING_YET);
}
