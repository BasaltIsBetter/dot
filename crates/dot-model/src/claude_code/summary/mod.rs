use std::io::{self, BufReader};
use std::path::Path;

use dot_api::{Cancellation, Prompt, ReplyError, ReplyEvent, Summaries};
use dot_process::read_line;

use crate::claude_code::launch::{Running, command};
use crate::claude_code::listen::LONGEST_LINE;
use crate::claude_code::output::{Compaction, Line, parse};
use crate::claude_code::{ClaudeCodeChat, ClaudeCodeError, EXIT_GRACE, failed, hand_over};
use crate::pricing;

const SUMMARIZE: &str = "/compact";

impl ClaudeCodeChat {
    pub(super) fn summary_window(&self) -> Option<u64> {
        if self.summaries == Summaries::LeftToClaude {
            return None;
        }
        let price = pricing::price_of(self.model.id);
        self.ledger.window(&price, self.model.context_window)
    }

    pub(super) fn announce_summary(&mut self, on_event: &mut dyn FnMut(ReplyEvent)) {
        if let Some(Compaction { before, after }) = self.unannounced.take() {
            on_event(ReplyEvent::Compacted { before, after });
        }
    }

    pub(crate) fn settle(&mut self, project: Option<&Path>, cancellation: &Cancellation) {
        let price = pricing::price_of(self.model.id);
        if self.summaries == Summaries::LeftToClaude || !self.ledger.summary_is_due(&price) {
            return;
        }
        if let Err(error) = self.summarize(project, cancellation) {
            tracing::warn!(
                failure = error.failure().text().id(),
                "the conversation was not summarized between two prompts: the next reply has it done"
            );
        }
    }

    fn summarize(
        &mut self,
        project: Option<&Path>,
        cancellation: &Cancellation,
    ) -> Result<(), ReplyError> {
        let program = self
            .program
            .as_deref()
            .ok_or_else(|| failed(ClaudeCodeError::Missing))?;
        let folder = self.folder(project)?;
        let handed = self.handed(folder);
        let options = self.options(handed.as_ref(), None, None);
        let launch = command(program, folder, self.model.id, self.effort, options);
        let (running, pipes) = Running::start(launch, cancellation).map_err(failed)?;

        let mut stdin = pipes.stdin;
        let asked = Prompt {
            text: SUMMARIZE.to_owned(),
            images: Vec::new(),
        };
        let handed = hand_over(stdin.as_mut(), &asked);
        let summarized = pipes.stdout.map_or(Ok(None), heard_a_summary);
        drop(stdin);
        running.finish(EXIT_GRACE).map_err(failed)?;
        handed.map_err(failed)?;

        if let Some(compaction) = summarized.map_err(failed)? {
            self.ledger.summarized();
            self.unannounced = Some(compaction);
        }
        Ok(())
    }
}

fn heard_a_summary(stdout: impl io::Read) -> io::Result<Option<Compaction>> {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    let mut summarized = None;

    while let Some(read) = read_line(&mut reader, &mut line, LONGEST_LINE)? {
        if read == dot_process::Line::Cut {
            continue;
        }
        match parse(&line) {
            Ok(Line::System {
                compaction: Some(compaction),
                ..
            }) => summarized = Some(compaction),
            Ok(Line::Result(_)) => break,
            Ok(_) | Err(_) => {}
        }
    }
    Ok(summarized)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-model/claude_code/summary/mod.rs"]
mod tests;
