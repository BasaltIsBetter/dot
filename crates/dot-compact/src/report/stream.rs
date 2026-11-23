use crate::command::CommandLine;
use crate::noise;
use crate::note::NEW_LINE;
use crate::report::progress::{active, shortened};
use crate::report::rules::Rule;

const LONGEST_WAIT: usize = 64 * 1_024;

/// What shortens the output of a command while the command still runs.
///
/// Lines are told as soon as they cannot change any more, without the colours of the terminal.
/// A line the program redraws in place, such as a progress bar, is told once, as it was drawn
/// last. A run of progress lines of a tool the command names waits for the line that ends it,
/// or for [`LineStream::rest`], and becomes a count in square brackets. No more than 64 KiB
/// ever waits: past that, the run is told as it stands, and the line that has not ended stays
/// waiting unless it is the one that went past 64 KiB.
pub struct LineStream {
    rules: Vec<&'static Rule>,
    waiting: String,
}

impl LineStream {
    pub(crate) fn of(command: &CommandLine<'_>) -> Self {
        Self {
            rules: active(command),
            waiting: String::new(),
        }
    }

    /// Takes the next piece of the output and returns what can be told now.
    pub fn push(&mut self, piece: &str) -> String {
        self.waiting.push_str(piece);
        self.forget_what_was_drawn_over();
        let complete_end = self
            .waiting
            .rfind(NEW_LINE)
            .map_or(0, |end| end + NEW_LINE.len_utf8());
        let unfinished = self.waiting.split_off(complete_end);
        let complete = std::mem::replace(&mut self.waiting, unfinished);

        let clean = noise::clean(&complete);
        let lines: Vec<&str> = clean.lines().collect();
        let settled = (0..lines.len())
            .rev()
            .find(|&at| !self.rules.iter().any(|rule| rule.drops(&lines, at)))
            .map_or(0, |last_kept| last_kept + 1);
        let open = &lines[settled..];
        let open_length: usize = open
            .iter()
            .map(|line| line.len() + NEW_LINE.len_utf8())
            .sum();

        if open_length + self.waiting.len() > LONGEST_WAIT {
            let mut at_once = told(&self.rules, &lines);
            if self.waiting.len() > LONGEST_WAIT {
                at_once.push_str(&std::mem::take(&mut self.waiting));
            }
            return at_once;
        }
        if !open.is_empty() {
            let mut waiting = open.join("\n");
            waiting.push(NEW_LINE);
            self.waiting.insert_str(0, &waiting);
        }
        told(&self.rules, &lines[..settled])
    }

    /// Returns what was still waiting, when the output pauses or ends.
    pub fn rest(&mut self) -> String {
        let waiting = std::mem::take(&mut self.waiting);
        let (complete, unfinished) = waiting
            .rfind(NEW_LINE)
            .map_or(("", waiting.as_str()), |end| waiting.split_at(end + 1));
        let clean = noise::clean(complete);
        let lines: Vec<&str> = clean.lines().collect();
        format!("{}{unfinished}", told(&self.rules, &lines))
    }

    fn forget_what_was_drawn_over(&mut self) {
        let unfinished = self.waiting.rfind(NEW_LINE).map_or(0, |end| end + 1);
        let drawn_over = noise::drawn_over(&self.waiting[unfinished..]);
        self.waiting
            .replace_range(unfinished..unfinished + drawn_over, "");
    }
}

fn told(rules: &[&Rule], lines: &[&str]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    shortened(rules, lines).0
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/report/stream.rs"]
mod tests;
