mod pair;

use crate::code::build::pair::Run;
use crate::code::edit::{Edit, Shown};
use crate::code::{CODE_LINES_KEPT, ChangeHunk, ChangeLine, SHOWN_FOLDED};

const SHOWN_AROUND: usize = 2;
const FOLDED_AT_LEAST: usize = 2;
pub(super) const KEPT_LINES: usize = CODE_LINES_KEPT;

#[derive(Debug)]
enum Raw {
    Same(u64, String),
    Change(Shown),
}

pub(super) fn edit_of(hunks: Vec<ChangeHunk>) -> Edit {
    let mut lines = Vec::new();
    let mut folds = 0;
    let mut changes = 0;
    let mut kept = 0;
    let mut beyond = 0;
    let mut previous_end: Option<u64> = None;

    for hunk in hunks.into_iter().filter(|hunk| !hunk.lines.is_empty()) {
        let room = KEPT_LINES - kept;
        if room == 0 {
            beyond += count(hunk.lines.len());
            continue;
        }
        if let Some(end) = previous_end
            && hunk.new_start > end
        {
            lines.push(Shown::Gap {
                count: hunk.new_start - end,
            });
        }
        let total = hunk.lines.len();
        let (raw, end) = raw_lines(hunk, &mut changes, room);
        kept += total.min(room);
        beyond += count(total.saturating_sub(room));
        previous_end = Some(end);
        fold(raw, &mut lines, &mut folds);
    }
    if beyond > 0 {
        lines.push(Shown::Gap { count: beyond });
    }

    let width = digits(
        lines
            .iter()
            .map(Shown::last_number)
            .max()
            .unwrap_or_default(),
    );
    let hidden: u64 = lines
        .get(SHOWN_FOLDED..)
        .unwrap_or_default()
        .iter()
        .map(Shown::line_count)
        .sum();
    let appearing = if hidden > 0 {
        SHOWN_FOLDED
    } else {
        lines.len()
    };
    let changes_shown = lines[..appearing]
        .iter()
        .filter(|shown| shown.is_a_change())
        .count();
    Edit {
        lines,
        changes,
        folds,
        width,
        hidden,
        appearing,
        changes_shown,
    }
}

fn raw_lines(hunk: ChangeHunk, changes: &mut usize, room: usize) -> (Vec<Raw>, u64) {
    let mut number = hunk.new_start;
    let mut raw = Vec::new();
    let mut run = Run::default();
    for line in hunk.lines.into_iter().take(room).map(ChangeLine::kept) {
        match line {
            ChangeLine::Removed(old) => {
                if !run.added.is_empty() {
                    run.flush(&mut raw, changes, &mut number);
                }
                run.removed.push(old);
            }
            ChangeLine::Added(text) => run.added.push(text),
            ChangeLine::Same(text) => {
                run.flush(&mut raw, changes, &mut number);
                raw.push(Raw::Same(number, text));
                number = number.saturating_add(1);
            }
        }
    }
    run.flush(&mut raw, changes, &mut number);
    (raw, number)
}

fn next(changes: &mut usize) -> usize {
    let order = *changes;
    *changes += 1;
    order
}

fn fold(raw: Vec<Raw>, lines: &mut Vec<Shown>, folds: &mut usize) {
    let mut run: Vec<(u64, String)> = Vec::new();
    let mut after_a_change = false;
    for line in raw {
        match line {
            Raw::Same(number, text) => run.push((number, text)),
            Raw::Change(shown) => {
                close_run(
                    std::mem::take(&mut run),
                    (after_a_change, true),
                    lines,
                    folds,
                );
                lines.push(shown);
                after_a_change = true;
            }
        }
    }
    close_run(run, (after_a_change, false), lines, folds);
}

fn close_run(
    mut run: Vec<(u64, String)>,
    (after_a_change, before_a_change): (bool, bool),
    lines: &mut Vec<Shown>,
    folds: &mut usize,
) {
    let head = if after_a_change { SHOWN_AROUND } else { 0 };
    let tail = if before_a_change { SHOWN_AROUND } else { 0 };
    if run.len() < head + tail + FOLDED_AT_LEAST {
        lines.extend(run.into_iter().map(same));
        return;
    }
    let tail_lines = run.split_off(run.len() - tail);
    let folded = run.split_off(head);
    lines.extend(run.into_iter().map(same));
    lines.push(Shown::Fold {
        fold: *folds,
        lines: folded,
    });
    *folds += 1;
    lines.extend(tail_lines.into_iter().map(same));
}

fn same((number, text): (u64, String)) -> Shown {
    Shown::Same { number, text }
}

fn count(lines: usize) -> u64 {
    u64::try_from(lines).unwrap_or(u64::MAX)
}

pub(super) fn digits(number: u64) -> usize {
    number
        .checked_ilog10()
        .map_or(1, |power| power as usize + 1)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/code/build/mod.rs"]
mod tests;
