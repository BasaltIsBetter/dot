use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::claude_code::ClaudeCodeChat;
use crate::claude_code::launch::SessionId;
use crate::context::Ledger;

const SESSION: &str = "session";
const BILLED: &str = "billed";
const LEDGER: &str = "ledger";

#[derive(Debug, Serialize)]
pub(crate) struct Kept<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
    billed: u64,
    ledger: &'a Ledger,
}

impl ClaudeCodeChat {
    pub(crate) fn kept(&self) -> Kept<'_> {
        Kept {
            session: self.session.as_ref().map(SessionId::as_str),
            billed: self.billed,
            ledger: &self.ledger,
        }
    }

    pub(crate) fn take_up(&mut self, kept: &Value) {
        let Some(session) = kept
            .get(SESSION)
            .and_then(Value::as_str)
            .and_then(SessionId::parse)
        else {
            return;
        };
        self.session = Some(session);
        self.billed = kept.get(BILLED).and_then(Value::as_u64).unwrap_or_default();
        self.ledger = kept
            .get(LEDGER)
            .and_then(|ledger| Ledger::deserialize(ledger).ok())
            .unwrap_or(Ledger::NOTHING_YET);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/claude_code/memory.rs"]
mod tests;
