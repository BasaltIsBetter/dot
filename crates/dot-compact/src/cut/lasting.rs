use crate::cut::Limits;

const LINE_BREAK: u8 = b'\n';
const LONGEST_UNFINISHED_LINE: usize = 64 * 1_024;

/// What cuts an output while it still comes, burst by burst.
///
/// The start of a burst passes at once. Past it, only the end of the burst is kept, and the
/// lines between are counted: [`Burst::pause`] tells them, `[+312 lines cut]`, then what was
/// kept, and the next burst starts afresh. A burst no longer than a finished output may be is
/// told whole. Only whole lines are cut, nothing changes order, and a count always starts a line.
/// A line that has not ended waits for its end, for [`Burst::pause`], or for 64 KiB: past that,
/// what the burst kept is told first, then the start of the line, and the rest of that line
/// passes as it comes.
pub struct Burst {
    limits: &'static Limits,
    passed: usize,
    kept: Vec<u8>,
    unfinished: Vec<u8>,
    lines_cut: usize,
    has_cut: bool,
    mid_line: bool,
}

impl Burst {
    pub(crate) const fn within(limits: &'static Limits) -> Self {
        Self {
            limits,
            passed: 0,
            kept: Vec::new(),
            unfinished: Vec::new(),
            lines_cut: 0,
            has_cut: false,
            mid_line: false,
        }
    }

    /// Takes the next piece of the output and returns what passes now.
    pub fn pass(&mut self, piece: &[u8]) -> Vec<u8> {
        self.unfinished.extend_from_slice(piece);
        let finished_end = self
            .unfinished
            .iter()
            .rposition(|&byte| byte == LINE_BREAK)
            .map_or(0, |end| end + 1);
        let unfinished = self.unfinished.split_off(finished_end);
        let finished = std::mem::replace(&mut self.unfinished, unfinished);

        let mut passing = Vec::new();
        for line in finished.split_inclusive(|&byte| byte == LINE_BREAK) {
            self.take_line(line, &mut passing);
        }
        self.forget_the_middle();
        if self.unfinished.len() > LONGEST_UNFINISHED_LINE {
            self.pass_the_start_of_a_line(&mut passing);
        }
        passing
    }

    /// Returns what the burst held back, when the output pauses or ends: the count of the lines
    /// that were cut, the end of the burst, and the line that had not ended.
    pub fn pause(&mut self) -> Vec<u8> {
        let mut told = self.held();
        let unfinished = std::mem::take(&mut self.unfinished);
        self.tell(&mut told, &unfinished);
        self.passed = 0;
        told
    }

    /// Returns whether a burst lost lines since the output started, told yet or not.
    #[must_use]
    pub const fn has_cut(&self) -> bool {
        self.has_cut
    }

    /// Returns whether what the burst told last stops in the middle of a line, so that a note
    /// told after it has to start a line of its own.
    #[must_use]
    pub const fn is_mid_line(&self) -> bool {
        self.mid_line
    }

    fn take_line(&mut self, line: &[u8], passing: &mut Vec<u8>) {
        let passes = self.mid_line || self.passed < self.limits.kept_from_the_start;
        if !passes {
            self.kept.extend_from_slice(line);
            return;
        }
        self.passed += line.len();
        self.tell(passing, line);
    }

    fn pass_the_start_of_a_line(&mut self, passing: &mut Vec<u8>) {
        let held = self.held();
        passing.extend_from_slice(&held);
        let start = std::mem::take(&mut self.unfinished);
        self.passed += start.len();
        self.tell(passing, &start);
    }

    fn held(&mut self) -> Vec<u8> {
        let mut told = Vec::new();
        if self.lines_cut > 0 {
            let count = format!("[+{} lines cut]\n", self.lines_cut);
            self.tell(&mut told, count.as_bytes());
            self.lines_cut = 0;
        }
        let kept = std::mem::take(&mut self.kept);
        self.tell(&mut told, &kept);
        told
    }

    fn tell(&mut self, told: &mut Vec<u8>, bytes: &[u8]) {
        if let Some(&last) = bytes.last() {
            self.mid_line = last != LINE_BREAK;
        }
        told.extend_from_slice(bytes);
    }

    fn forget_the_middle(&mut self) {
        let most_kept = self.limits.cut_above - self.limits.kept_from_the_start;
        if self.kept.len() <= most_kept {
            return;
        }
        let kept_at_the_end = self.limits.kept - self.limits.kept_from_the_start;
        let wanted = self.kept.len() - kept_at_the_end;
        let last_line = self.kept[..self.kept.len() - 1]
            .iter()
            .rposition(|&byte| byte == LINE_BREAK)
            .map_or(0, |end| end + 1);
        let from = self.kept[wanted - 1..]
            .iter()
            .position(|&byte| byte == LINE_BREAK)
            .map_or(last_line, |end| (wanted + end).min(last_line));

        self.lines_cut += self.kept[..from]
            .split_inclusive(|&byte| byte == LINE_BREAK)
            .count();
        self.has_cut |= from > 0;
        self.kept.drain(..from);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/cut/lasting.rs"]
mod tests;
