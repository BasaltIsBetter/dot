mod list;

use std::time::Instant;

use crate::conversation::flow::Conversation;
use crate::conversation::prompter::{CaretAt, Motion};

pub(crate) use self::list::Recall;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Recalling {
    at: usize,
    draft: String,
    caret: usize,
}

impl Conversation {
    pub(crate) fn recall(&mut self, motion: Motion, recall: &Recall, now: Instant) -> bool {
        if self.question.is_some() || self.agent_view.is_some() {
            return false;
        }
        let (row, _) = self.prompter.caret_cell();
        let recalled = match motion {
            Motion::Up if row == 0 => self.recall_older(recall),
            Motion::Down if row >= self.prompter.last_row() => self.recall_newer(recall),
            _ => false,
        };
        if recalled {
            self.blink.restart(now);
            self.help.stir(now);
            self.reader.come_back();
        }
        recalled
    }

    fn recall_older(&mut self, recall: &Recall) -> bool {
        let than = self.recalling.as_ref().map(|recalling| recalling.at);
        let Some((at, prompt)) = recall.older(than) else {
            return false;
        };
        let recalling = self.recalling.get_or_insert_with(|| Recalling {
            at,
            draft: self.prompter.text().to_owned(),
            caret: self.prompter.caret_offset(),
        });
        recalling.at = at;
        self.prompter.show(prompt, CaretAt::EndOfFirstRow);
        true
    }

    fn recall_newer(&mut self, recall: &Recall) -> bool {
        let Some(recalling) = self.recalling.as_mut() else {
            return false;
        };
        if let Some((at, prompt)) = recall.newer(recalling.at) {
            recalling.at = at;
            self.prompter.show(prompt, CaretAt::End);
            return true;
        }
        let Some(Recalling { draft, caret, .. }) = self.recalling.take() else {
            return false;
        };
        self.prompter.show(&draft, CaretAt::Offset(caret));
        true
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/recall/mod.rs"]
mod tests;
