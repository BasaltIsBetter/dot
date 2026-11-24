use std::mem;
use std::time::Instant;

use crate::line::{TextWidth, columns, push_truncated};
use crate::transcript::entry::{Entry, LONGEST_TEXT, floor_char_boundary};
use crate::transcript::tool::{Aside, Glyph, Replaced, Summary};
use crate::wrap::normalized;

impl Entry {
    pub(crate) fn runs_a_command(&self) -> bool {
        self.mark
            .as_deref()
            .is_some_and(|mark| mark.tool.glyph == Glyph::Command)
    }

    pub(crate) fn sums_up_a_command(&self) -> bool {
        self.runs_a_command() && self.code().is_none() && !self.text.is_empty()
    }

    pub(crate) fn sum_up(&mut self, aside: Aside<'_>, now: Instant, width: TextWidth) -> bool {
        let Some(mark) = self.mark.as_deref_mut() else {
            return false;
        };
        let summary = Summary::of(aside);
        let mut fitted = String::new();
        summary.fit_into(width, &mut fitted);
        mark.summary = Some(Box::new(summary));
        self.extend(&fitted, now, width)
    }

    pub(crate) fn replace_aside(
        &mut self,
        aside: Aside<'_>,
        now: Instant,
        width: TextWidth,
    ) -> bool {
        let cut = normalized(aside.cut, usize::MAX);
        let kept = normalized(aside.kept, usize::MAX);
        let summary = self
            .mark
            .as_deref_mut()
            .and_then(|mark| mark.summary.as_deref_mut())
            .filter(|_| !cut.is_empty() || !kept.is_empty());
        let Some(summary) = summary else {
            return false;
        };
        summary.cut.clear();
        summary.cut.push_str(&cut);
        summary.kept.clear();
        summary.kept.push_str(&kept);
        let replaced = summary.replaced.get_or_insert_with(|| Replaced {
            before: String::new(),
            at: now,
        });
        replaced.at = now;
        mem::swap(&mut self.text, &mut replaced.before);
        self.text.clear();
        summary.fit_into(width, &mut self.text);
        self.arrivals.clear();
        self.rows.clear();
        self.rewrap_tail(width);
        true
    }

    pub(crate) fn summary(&self) -> Option<&Summary> {
        self.mark.as_deref()?.summary.as_deref()
    }

    pub(crate) fn replaced(&self) -> Option<&Replaced> {
        self.summary()?.replaced.as_ref()
    }
}

impl Summary {
    fn of(aside: Aside<'_>) -> Self {
        Self {
            cut: normalized(aside.cut, usize::MAX).into_owned(),
            kept: normalized(aside.kept, usize::MAX).into_owned(),
            replaced: None,
        }
    }

    pub(crate) fn fit_into(&self, width: TextWidth, fitted: &mut String) {
        let room = width.columns().saturating_sub(columns(&self.kept));
        push_truncated(fitted, &self.cut, room);
        fitted.push_str(&self.kept);
        fitted.truncate(floor_char_boundary(fitted, LONGEST_TEXT));
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/transcript/entry/aside.rs"]
mod tests;
