use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::messages::MessagesChat;

const LIST_START: u8 = b'[';
const LIST_END: u8 = b']';

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeptMessage {
    role: Role,
    #[serde(rename = "content")]
    _content: IgnoredAny,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    User,
    Assistant,
}

impl MessagesChat {
    pub(crate) fn history(&self) -> &[u8] {
        &self.history
    }

    pub(crate) fn take_up(&mut self, history: Vec<u8>) {
        if is_a_conversation(&history) {
            self.history = history;
        }
    }
}

fn is_a_conversation(history: &[u8]) -> bool {
    let mut listed = Vec::with_capacity(history.len() + 2);
    listed.push(LIST_START);
    listed.extend_from_slice(history);
    listed.push(LIST_END);
    serde_json::from_slice::<Vec<KeptMessage>>(&listed).is_ok_and(|messages| {
        messages
            .first()
            .is_some_and(|first| first.role == Role::User)
    })
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/messages/memory.rs"]
mod tests;
