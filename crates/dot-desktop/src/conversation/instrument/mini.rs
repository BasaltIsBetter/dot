use crate::canvas::{Paint, Rectangle};
use crate::conversation::stage::Stage;
use dot_transcript::{Ink, SPARK_COLUMNS};

pub(crate) const MINI_SPARK_HEIGHT: f32 = 10.0;
pub(crate) const MINI_SPARK_WIDTH: f32 = 84.0;
const SPARK_SHOWN: usize = 16;
const SPARK_CELLS: u8 = 3;
const TOKENS_PER_SECOND_PER_CELL: f64 = 28.8;

pub(crate) fn mini_spark(
    rates: &[u16; SPARK_COLUMNS],
    [left, top]: [f32; 2],
    stage: &Stage,
    live: Option<Ink>,
) -> Vec<Rectangle> {
    let scale = |points: f32| stage.exact_points(points);
    let shown = &rates[SPARK_COLUMNS - SPARK_SHOWN..];
    let gaps = to_points(SPARK_SHOWN - 1);
    let column_width = (MINI_SPARK_WIDTH - gaps) / to_points(SPARK_SHOWN);
    let cell_height = (MINI_SPARK_HEIGHT - f32::from(SPARK_CELLS - 1)) / f32::from(SPARK_CELLS);
    let bottom = top + scale(MINI_SPARK_HEIGHT);
    let mut cells = Vec::with_capacity(SPARK_SHOWN * usize::from(SPARK_CELLS));
    for (column, &rate) in shown.iter().enumerate() {
        let lit = lit_cells(rate);
        let newest = column + 1 == SPARK_SHOWN;
        let column_left = left + scale(to_points(column) * (column_width + 1.0));
        for cell in 0..SPARK_CELLS {
            let ink = if cell < lit {
                match live.filter(|_| newest) {
                    Some(live) => live,
                    None => Ink::Receding,
                }
            } else if cell == 0 {
                Ink::Faint
            } else {
                Ink::Trace
            };
            let cell_top = bottom - scale(f32::from(cell) * (cell_height + 1.0) + cell_height);
            let [x, y] = [column_left.round(), cell_top.round()];
            cells.push(Rectangle {
                left: x,
                top: y,
                width: (column_left + scale(column_width)).round() - x,
                height: (cell_top + scale(cell_height)).round() - y,
                color: Paint::ink(ink),
            });
        }
    }
    cells
}

fn lit_cells(rate: u16) -> u8 {
    if rate == 0 {
        return 0;
    }
    let cells = (f64::from(rate) / TOKENS_PER_SECOND_PER_CELL)
        .round()
        .clamp(1.0, f64::from(SPARK_CELLS));
    to_level(cells)
}

fn to_points(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a count of cells lies within the 0 to 255 range"
)]
fn to_level(level: f64) -> u8 {
    level.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/instrument/mini.rs"]
mod tests;
