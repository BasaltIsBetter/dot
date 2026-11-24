use dot_transcript::NEW_LINE;

const MOST_EDITS: usize = 700;
const MOST_COMPARISONS: usize = 4_000_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Lines {
    pub(crate) changed: Vec<bool>,
    pub(crate) removed: usize,
}

impl Lines {
    pub(crate) fn count(&self) -> usize {
        let changed = self.changed.iter().filter(|&&changed| changed).count();
        changed.max(self.removed)
    }
}

pub(crate) fn changed_lines(before: &str, after: &str) -> Lines {
    let old: Vec<&str> = before.split(NEW_LINE).collect();
    let new: Vec<&str> = after.split(NEW_LINE).collect();
    let head = old
        .iter()
        .zip(&new)
        .take_while(|(old, new)| old == new)
        .count();
    let tail = old[head..]
        .iter()
        .rev()
        .zip(new[head..].iter().rev())
        .take_while(|(old, new)| old == new)
        .count();
    let old_middle = &old[head..old.len() - tail];
    let new_middle = &new[head..new.len() - tail];
    let mut changed = vec![false; new.len()];
    let kept = if let Some(kept) = common(old_middle, new_middle) {
        for (flag, kept) in changed[head..].iter_mut().zip(&kept) {
            *flag = !kept;
        }
        kept.iter().filter(|&&kept| kept).count()
    } else {
        changed[head..new.len() - tail].fill(true);
        0
    };
    Lines {
        changed,
        removed: old_middle.len() - kept,
    }
}

fn common(old: &[&str], new: &[&str]) -> Option<Vec<bool>> {
    let frontiers = frontiers(old, new, MOST_COMPARISONS)?;
    let mut kept = vec![false; new.len()];
    let (mut x, mut y) = (old.len(), new.len());
    for (edits, previous) in frontiers.windows(2).enumerate().rev() {
        let edits = edits + 1;
        let diagonal = diagonal_of(x, y);
        let from = step_from(&previous[0], diagonal, edits);
        let start = previous[0][index_of(from, edits - 1)];
        let start = (start, along(start, from));
        let (after_x, after_y) = if from > diagonal {
            (start.0, start.1 + 1)
        } else {
            (start.0 + 1, start.1)
        };
        while x > after_x && y > after_y {
            x -= 1;
            y -= 1;
            kept[y] = true;
        }
        (x, y) = start;
    }
    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        kept[y] = true;
    }
    Some(kept)
}

fn frontiers(old: &[&str], new: &[&str], most_compared: usize) -> Option<Vec<Vec<usize>>> {
    let mut frontiers: Vec<Vec<usize>> = Vec::new();
    let mut compared = 0_usize;
    for edits in 0..=MOST_EDITS {
        let mut frontier = vec![0; 2 * edits + 1];
        for diagonal in (-signed(edits)..=signed(edits)).step_by(2) {
            let mut x = match frontiers.last() {
                None => 0,
                Some(previous) => {
                    let from = step_from(previous, diagonal, edits);
                    let start = previous[index_of(from, edits - 1)];
                    if from > diagonal { start } else { start + 1 }
                }
            };
            let mut y = along(x, diagonal);
            while x < old.len() && y < new.len() && old[x] == new[y] {
                x += 1;
                y += 1;
                compared += 1;
            }
            compared += 1;
            frontier[index_of(diagonal, edits)] = x;
            if x >= old.len() && y >= new.len() {
                frontiers.push(frontier);
                return Some(frontiers);
            }
        }
        if compared > most_compared {
            return None;
        }
        frontiers.push(frontier);
    }
    None
}

fn step_from(previous: &[usize], diagonal: isize, edits: usize) -> isize {
    let edits = signed(edits);
    let below =
        |diagonal: isize| previous[index_of(diagonal, usize::try_from(edits - 1).unwrap_or(0))];
    if diagonal == -edits || (diagonal != edits && below(diagonal - 1) < below(diagonal + 1)) {
        diagonal + 1
    } else {
        diagonal - 1
    }
}

fn index_of(diagonal: isize, edits: usize) -> usize {
    usize::try_from(diagonal + signed(edits)).unwrap_or(0)
}

fn along(x: usize, diagonal: isize) -> usize {
    usize::try_from(signed(x) - diagonal).unwrap_or(0)
}

fn diagonal_of(x: usize, y: usize) -> isize {
    signed(x) - signed(y)
}

fn signed(count: usize) -> isize {
    isize::try_from(count).unwrap_or(isize::MAX)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/versions/changed.rs"]
mod tests;
