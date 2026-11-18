use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    RowStart,
    RowEnd,
    Up,
    Down,
    Start,
    End,
}

pub(crate) struct Cursor<'a> {
    pub(crate) text: &'a str,
    pub(crate) rows: &'a [Range<usize>],
    pub(crate) caret: Spot,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Spot {
    pub(crate) offset: usize,
    pub(crate) upstream: bool,
}

impl Spot {
    pub(crate) const fn at(offset: usize) -> Self {
        Self {
            offset,
            upstream: false,
        }
    }
}

impl Cursor<'_> {
    pub(crate) fn target(&self, motion: Motion, goal: usize) -> Spot {
        let row = self.row();
        let offset = self.caret.offset;
        match motion {
            Motion::Left => Spot::at(
                self.text[..offset]
                    .chars()
                    .next_back()
                    .map_or(offset, |character| offset - character.len_utf8()),
            ),
            Motion::Right => Spot::at(
                self.text[offset..]
                    .chars()
                    .next()
                    .map_or(offset, |character| offset + character.len_utf8()),
            ),
            Motion::WordLeft => Spot::at(self.word_left()),
            Motion::WordRight => Spot::at(self.word_right()),
            Motion::RowStart => Spot::at(self.rows[row].start),
            Motion::RowEnd => self.spot_at(row, usize::MAX),
            Motion::Up if row == 0 => Spot::at(0),
            Motion::Up => self.spot_at(row - 1, goal),
            Motion::Down if row + 1 == self.rows.len() => Spot::at(self.text.len()),
            Motion::Down => self.spot_at(row + 1, goal),
            Motion::Start => Spot::at(0),
            Motion::End => Spot::at(self.text.len()),
        }
    }

    pub(crate) fn row(&self) -> usize {
        let offset = self.caret.offset;
        let row = self
            .rows
            .partition_point(|row| row.start <= offset)
            .saturating_sub(1);
        let ends_the_row_above = row > 0 && self.rows[row - 1].end == offset;
        if self.caret.upstream && ends_the_row_above {
            return row - 1;
        }
        row
    }

    pub(crate) fn column(&self) -> usize {
        let start = self.rows[self.row()].start;
        self.text[start..self.caret.offset].chars().count()
    }

    pub(crate) fn spot_at(&self, row: usize, column: usize) -> Spot {
        let row = row.min(self.rows.len() - 1);
        let range = self.rows[row].clone();
        let wrapped = self
            .rows
            .get(row + 1)
            .is_some_and(|next| next.start == range.end);

        self.text[range.clone()].char_indices().nth(column).map_or(
            Spot {
                offset: range.end,
                upstream: wrapped,
            },
            |(offset, _)| Spot::at(range.start + offset),
        )
    }

    fn word_left(&self) -> usize {
        let before = &self.text[..self.caret.offset];
        let word_end = last_after(before, is_word);
        last_after(&before[..word_end], |character| !is_word(character))
    }

    fn word_right(&self) -> usize {
        let after = &self.text[self.caret.offset..];
        let word_start = after.find(is_word).unwrap_or(after.len());
        let word_end = after[word_start..]
            .find(|character| !is_word(character))
            .map_or(after.len(), |length| word_start + length);
        self.caret.offset + word_end
    }
}

fn last_after(text: &str, matches: impl Fn(char) -> bool) -> usize {
    text.char_indices()
        .rev()
        .find(|&(_, character)| matches(character))
        .map_or(0, |(offset, character)| offset + character.len_utf8())
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/prompter/motion.rs"]
mod tests;
