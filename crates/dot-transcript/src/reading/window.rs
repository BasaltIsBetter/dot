/// The rows a view shows: from `first` to `end`, the first `offset` rows above its top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Window {
    /// First row shown, whole or in part.
    pub(crate) first: usize,
    /// Row past the last one shown.
    pub(crate) end: usize,
    /// How far the first row starts above the view, in rows, from -1 to 0.
    pub(crate) offset: f64,
}

impl Window {
    /// Returns the rows a view of `visible` rows shows from `top`, any row, even negative,
    /// within `total` rows.
    #[must_use]
    pub(crate) fn at(top: f64, total: usize, visible: f64) -> Self {
        let start = top.floor().max(0.0);
        let end = index((top + visible.max(0.0)).ceil()).min(total);
        Self {
            first: index(start).min(end),
            end,
            offset: start - top,
        }
    }
}

/// Returns a count of rows as a number of rows that can be part of one.
#[must_use]
pub fn rows(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Returns the whole row that holds `row`, zero for a row above the first.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the row is floored at zero first, and a float cast to usize saturates"
)]
#[must_use]
pub fn index(row: f64) -> usize {
    row.max(0.0) as usize
}

/// Returns `value` as an `f32`, for pixels and rows that stay far within its range.
#[expect(
    clippy::cast_possible_truncation,
    reason = "scale factors, window coordinates and rows stay far below the range of f32"
)]
#[must_use]
pub fn to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
#[path = "../../../../tests/dot-transcript/reading/window.rs"]
mod tests;
