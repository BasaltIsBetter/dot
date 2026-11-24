use crate::code::CodeBlock;
use crate::reading::rows;
use crate::transcript::history::{Insertion, Transcript};

impl Transcript {
    pub(super) fn stack_code(&mut self, position: usize) -> Option<usize> {
        let mut run = Vec::new();
        let mut later = position;
        while let Some(earlier) = self
            .visible_before(later)
            .filter(|&earlier| self.entries[earlier].code().is_some())
        {
            run.push(earlier);
            later = earlier;
        }
        let &previous = run.first()?;
        let pad = run
            .iter()
            .filter_map(|&at| self.entries[at].code())
            .map(CodeBlock::header_columns)
            .max()
            .unwrap_or_default();
        let rows_before = self.entries[previous].row_count();
        for &at in &run {
            let entry = &mut self.entries[at];
            if let Some(code) = entry.code_mut() {
                code.stack(pad);
            }
            entry.recount_code();
        }
        let folded = rows_before.saturating_sub(self.entries[previous].row_count());
        if let Some(first) = self.first_row_of(previous) {
            self.insertions.push(Insertion {
                at: rows(first + rows_before),
                rows: -rows(folded),
            });
        }
        run.last().copied()
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-transcript/transcript/history/tool/stack.rs"]
mod tests;
