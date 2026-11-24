use std::mem;

use crate::code::build::{Raw, next};
use crate::code::edit::Shown;

const LEAST_ALIKE: u32 = 300;
const WHOLLY_ALIKE: u32 = 1_000;
const MOST_COMPARED: usize = 10_000;

#[derive(Debug, Default)]
pub(super) struct Run {
    pub(super) removed: Vec<String>,
    pub(super) added: Vec<String>,
}

impl Run {
    pub(super) fn flush(&mut self, raw: &mut Vec<Raw>, changes: &mut usize, number: &mut u64) {
        let pairs = pairs(&self.removed, &self.added);
        let mut removed = mem::take(&mut self.removed)
            .into_iter()
            .enumerate()
            .peekable();
        let mut added = mem::take(&mut self.added)
            .into_iter()
            .enumerate()
            .peekable();
        let mut change = |shown| raw.push(Raw::Change(shown));
        for (old_at, new_at) in pairs {
            while let Some((_, old)) = removed.next_if(|(at, _)| *at < old_at) {
                change(removed_line(old, changes));
            }
            while let Some((_, text)) = added.next_if(|(at, _)| *at < new_at) {
                change(added_line(number, text, changes));
            }
            if let (Some((_, old)), Some((_, text))) = (removed.next(), added.next()) {
                change(Shown::Changed {
                    number: take_number(number),
                    text,
                    old,
                    order: next(changes),
                });
            }
        }
        for (_, old) in removed {
            change(removed_line(old, changes));
        }
        for (_, text) in added {
            change(added_line(number, text, changes));
        }
    }
}

fn removed_line(old: String, changes: &mut usize) -> Shown {
    Shown::Removed {
        old,
        order: next(changes),
    }
}

fn added_line(number: &mut u64, text: String, changes: &mut usize) -> Shown {
    Shown::Added {
        number: take_number(number),
        text,
        order: next(changes),
    }
}

fn take_number(number: &mut u64) -> u64 {
    let taken = *number;
    *number = number.saturating_add(1);
    taken
}

fn pairs(old: &[String], new: &[String]) -> Vec<(usize, usize)> {
    let alike = if old.len().saturating_mul(new.len()) > MOST_COMPARED {
        Vec::new()
    } else {
        most_alike(old, new)
    };
    with_the_rest_in_order(&alike, old.len(), new.len())
}

fn most_alike(old: &[String], new: &[String]) -> Vec<(usize, usize)> {
    let width = new.len() + 1;
    let mut best = vec![0_u32; (old.len() + 1) * width];
    for (row, before) in old.iter().enumerate() {
        for (column, after) in new.iter().enumerate() {
            let skipped = best[row * width + column + 1].max(best[(row + 1) * width + column]);
            let alike = likeness(before, after);
            let taken = if alike >= LEAST_ALIKE {
                best[row * width + column] + alike
            } else {
                0
            };
            best[(row + 1) * width + column + 1] = skipped.max(taken);
        }
    }

    let mut pairs = Vec::new();
    let (mut row, mut column) = (old.len(), new.len());
    while let (Some(above), Some(before)) = (row.checked_sub(1), column.checked_sub(1)) {
        let here = best[row * width + column];
        if here == best[above * width + column] {
            row = above;
        } else if here == best[row * width + before] {
            column = before;
        } else {
            pairs.push((above, before));
            (row, column) = (above, before);
        }
    }
    pairs.reverse();
    pairs
}

fn with_the_rest_in_order(alike: &[(usize, usize)], old: usize, new: usize) -> Vec<(usize, usize)> {
    let mut pairs = Vec::with_capacity(old.min(new));
    let (mut row, mut column) = (0, 0);
    for &(paired_row, paired_column) in alike.iter().chain([&(old, new)]) {
        let between = (paired_row - row).min(paired_column - column);
        pairs.extend((0..between).map(|step| (row + step, column + step)));
        if paired_row < old {
            pairs.push((paired_row, paired_column));
        }
        row = paired_row + 1;
        column = paired_column + 1;
    }
    pairs
}

fn likeness(old: &str, new: &str) -> u32 {
    let (old, new) = (old.trim_start(), new.trim_start());
    let (old_count, new_count) = (old.chars().count(), new.chars().count());
    let longest = old_count.max(new_count);
    if longest == 0 {
        return WHOLLY_ALIKE;
    }
    let head = old
        .chars()
        .zip(new.chars())
        .take_while(|(left, right)| left == right)
        .count();
    let tail = old
        .chars()
        .rev()
        .zip(new.chars().rev())
        .take(old_count.min(new_count) - head)
        .take_while(|(left, right)| left == right)
        .count();
    per_mille(head + tail, longest)
}

fn per_mille(part: usize, whole: usize) -> u32 {
    let share = part.saturating_mul(1_000) / whole.max(1);
    u32::try_from(share).unwrap_or(WHOLLY_ALIKE)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-transcript/code/build/pair.rs"]
mod tests;
