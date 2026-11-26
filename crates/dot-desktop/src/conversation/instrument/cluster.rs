use dot_api::{Text, Translator};

use crate::canvas::{Layout, Paint, Pane, Rectangle, TextSize};
use crate::conversation::instrument::Reading;
use crate::conversation::stage::Stage;
use crate::status_bar::seconds;
use dot_transcript::{
    Ink, Line, Measures, Placed, SPARK_CELLS, SPARK_COLUMNS, Span, Spark, columns, opacity,
};

const RIGHT_OF_THE_COLUMN: f32 = 24.0;
pub(super) const VALUE_COLUMNS: usize = 7;
const COLUMN_GAP: f32 = 12.0;
const ROW: f32 = 22.0;
const ROWS: usize = 3;
const FIRST_ROW_ABOVE_THE_BAND_CENTRE: f32 = 33.0;
const SPARK_WIDTH: f32 = 79.0;
const SPARK_HEIGHT: f32 = 14.0;
const SPARK_COLUMN_WIDTH: f32 = 3.0;
const SPARK_COLUMN_STEP: f32 = 4.0;
const SPARK_CELL_HEIGHT: f32 = 2.0;
const SPARK_CELL_STEP: f32 = 3.0;
const LABEL_INK: Ink = Ink::Secondary;
const VALUE_INK: Ink = Ink::Full;
const DARK_CELL_INK: Ink = Ink::Trace;
const FLOOR_CELL_INK: Ink = Ink::Faint;
const LIT_CELL_INK: Ink = Ink::Receding;
const PLACEHOLDER: &str = "–––";
const DIGIT_PLACEHOLDER: &str = "–";
const SHORT_SEPARATOR: &str = " · ";

pub(crate) struct Cluster {
    pub(crate) panes: [Pane; 2],
    pub(crate) marks: Vec<Rectangle>,
    pub(crate) grid: Grid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Grid {
    pub(crate) label_right: f32,
    pub(crate) value_left: f32,
    pub(crate) first_top: f32,
    pub(crate) row: f32,
}

impl Cluster {
    pub(crate) fn of(
        reading: &Reading,
        band: &Rectangle,
        stage: &Stage,
        layout: &Layout,
        translator: &impl Translator,
    ) -> Self {
        let advance = layout.advance_of(TextSize::Small);
        let row = stage.points(ROW);
        let first_top =
            (band.top + band.height / 2.0 - stage.points(FIRST_ROW_ABOVE_THE_BAND_CENTRE)).round();
        let tops: [f32; ROWS] = std::array::from_fn(|index| {
            let top = first_top + row * f32::from(u8::try_from(index).unwrap_or(u8::MAX));
            layout.text_top(TextSize::Small, top, row)
        });
        let value_right = stage.column[0] - stage.points(RIGHT_OF_THE_COLUMN);
        let value_left =
            value_right - f32::from(u16::try_from(VALUE_COLUMNS).unwrap_or(0)) * advance;
        let label_right = value_left - stage.points(COLUMN_GAP);

        let labels = [Text::InstrumentTokens, Text::InstrumentTime]
            .map(|label| translator.translate(label).into_owned());
        let widest = labels
            .iter()
            .map(|label| columns(label))
            .max()
            .unwrap_or_default();
        let label_lines = [(0, &labels[0]), (2, &labels[1])].map(|(row, label)| Placed {
            top: tops[row],
            line: Line::new(
                indent(widest - columns(label)),
                vec![Span::new(label.as_str(), LABEL_INK)],
            )
            .faded(share(reading.alpha)),
            clip: None,
        });
        let values = texts(reading.measures.as_ref(), translator);
        let value_alpha = share(reading.alpha) * share(reading.values_alpha);
        let value_lines = values.iter().zip(tops).map(|(value, top)| Placed {
            top,
            line: Line::new(
                indent(VALUE_COLUMNS.saturating_sub(columns(value))),
                vec![Span::new(value.as_str(), VALUE_INK).heated(reading.heat)],
            )
            .faded(value_alpha),
            clip: None,
        });

        let spark_top = (first_top + row + (row - stage.points(SPARK_HEIGHT)) / 2.0).round();
        let spark = reading
            .measures
            .map_or_else(Spark::default, |measures| measures.spark);
        let marks = spark_rectangles(
            &spark,
            [label_right, spark_top],
            stage,
            opacity(value_alpha),
        )
        .filter(|cell| cell.top + cell.height <= stage.floor)
        .collect();

        let pane = |left: f32, lines: Vec<Placed>| Pane {
            left,
            size: TextSize::Small,
            top: 0.0,
            bottom: stage.floor,
            lines,
        };
        let widest_width = f32::from(u16::try_from(widest).unwrap_or(u16::MAX)) * advance;
        Self {
            panes: [
                pane(label_right - widest_width, label_lines.into()),
                pane(value_left, value_lines.collect()),
            ],
            marks,
            grid: Grid {
                label_right,
                value_left,
                first_top,
                row,
            },
        }
    }
}

pub(crate) fn short(measures: &Measures, translator: &impl Translator) -> String {
    let [tokens, rate, elapsed] = texts(Some(measures), translator);
    let word = translator.count(Text::UnitTokens, measures.tokens);
    [rate, format!("{tokens} {word}"), elapsed].join(SHORT_SEPARATOR)
}

fn texts(measures: Option<&Measures>, translator: &impl Translator) -> [String; ROWS] {
    let per_second = translator.translate(Text::UnitTokensPerSecond);
    let Some(measures) = measures else {
        let separator = translator.translate(Text::DecimalSeparator);
        let unit = translator.translate(Text::UnitSeconds);
        return [
            PLACEHOLDER.to_owned(),
            format!("{PLACEHOLDER} {per_second}"),
            format!("{DIGIT_PLACEHOLDER}{separator}{DIGIT_PLACEHOLDER} {unit}"),
        ];
    };
    let rate = measures
        .rate
        .map_or_else(|| PLACEHOLDER.to_owned(), |rate| rate.to_string());
    [
        measures.tokens.to_string(),
        format!("{rate} {per_second}"),
        seconds(measures.elapsed, translator),
    ]
}

fn spark_rectangles(
    spark: &Spark,
    [right, top]: [f32; 2],
    stage: &Stage,
    alpha: u8,
) -> impl Iterator<Item = Rectangle> {
    let scale = |points: f32| stage.exact_points(points);
    let left = right - scale(SPARK_WIDTH);
    let bottom = top + scale(SPARK_HEIGHT);
    let spark = *spark;
    (0..SPARK_COLUMNS).flat_map(move |column| {
        let lit = spark.cells[column];
        let newest = column + 1 == SPARK_COLUMNS;
        let column_left = left + scale(SPARK_COLUMN_STEP * to_points(column));
        (0..SPARK_CELLS).map(move |cell| {
            let ink = if cell < lit {
                LIT_CELL_INK
            } else if cell == 0 {
                FLOOR_CELL_INK
            } else {
                DARK_CELL_INK
            };
            let heat = if newest && cell < lit { spark.flash } else { 0 };
            let cell_top = bottom - scale(SPARK_CELL_STEP * f32::from(cell) + SPARK_CELL_HEIGHT);
            let [x, y] = [column_left.round(), cell_top.round()];
            Rectangle {
                left: x,
                top: y,
                width: (column_left + scale(SPARK_COLUMN_WIDTH)).round() - x,
                height: (cell_top + scale(SPARK_CELL_HEIGHT)).round() - y,
                color: Paint::shaded(ink, heat, alpha),
            }
        })
    })
}

fn indent(columns: usize) -> u16 {
    u16::try_from(columns).unwrap_or(u16::MAX)
}

pub(super) fn share(alpha: u8) -> f64 {
    f64::from(alpha) / f64::from(u8::MAX)
}

fn to_points(column: usize) -> f32 {
    f32::from(u16::try_from(column).unwrap_or(u16::MAX))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/instrument/cluster.rs"]
mod tests;
