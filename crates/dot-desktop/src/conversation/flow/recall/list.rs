pub(crate) const MOST_PROMPTS: usize = 1_000;
pub(crate) const LARGEST_PROMPT: usize = 64 * 1_024;
pub(crate) const LARGEST_HISTORY: usize = 1_024 * 1_024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Recall {
    prompts: Vec<String>,
    bytes: usize,
    changes: u64,
}

impl Recall {
    pub(crate) fn add(&mut self, prompt: &str) {
        if !is_kept(prompt) {
            return;
        }
        if self.prompts.last().is_some_and(|last| last == prompt) {
            return;
        }
        if let Some(at) = self.prompts.iter().position(|kept| kept == prompt) {
            self.bytes -= self.prompts.remove(at).len();
        }
        self.prompts.push(prompt.to_owned());
        self.bytes += prompt.len();
        self.trim();
        self.changes += 1;
    }

    pub(crate) fn add_older(&mut self, older: Vec<String>) {
        let newer = std::mem::take(&mut self.prompts);
        self.bytes = 0;
        for prompt in older.into_iter().chain(newer) {
            if !is_kept(&prompt) {
                continue;
            }
            if let Some(at) = self.prompts.iter().position(|kept| *kept == prompt) {
                self.bytes -= self.prompts.remove(at).len();
            }
            self.bytes += prompt.len();
            self.prompts.push(prompt);
        }
        self.trim();
    }

    pub(crate) fn older(&self, than: Option<usize>) -> Option<(usize, &str)> {
        let at = than
            .unwrap_or(self.prompts.len())
            .min(self.prompts.len())
            .checked_sub(1)?;
        Some((at, &self.prompts[at]))
    }

    pub(crate) fn newer(&self, than: usize) -> Option<(usize, &str)> {
        let at = than.checked_add(1)?;
        self.prompts.get(at).map(|prompt| (at, prompt.as_str()))
    }

    pub(crate) fn prompts(&self) -> &[String] {
        &self.prompts
    }

    pub(crate) const fn changes(&self) -> u64 {
        self.changes
    }

    fn trim(&mut self) {
        let mut first = 0;
        while self.prompts.len() - first > MOST_PROMPTS || self.bytes > LARGEST_HISTORY {
            self.bytes -= self.prompts[first].len();
            first += 1;
        }
        self.prompts.drain(..first);
    }
}

fn is_kept(prompt: &str) -> bool {
    !prompt.trim().is_empty() && prompt.len() <= LARGEST_PROMPT
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/recall/list.rs"]
mod tests;
