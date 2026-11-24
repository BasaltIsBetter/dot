use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::{Layout, Rectangle, TextSize};
use crate::conversation::flow::reply::name_of;
use crate::conversation::flow::running::columns_of;
use crate::conversation::stage::Stage;
use crate::conversation::{MINI_SPARK_HEIGHT, MINI_SPARK_WIDTH, mini_spark};
use dot_transcript::{Agent, Ink, Line, Placed, Span, index, truncated};

const RULE: &str = "──";
const NAME_COLUMNS: usize = 8;
const ENTER: char = '↗';
const PLACEHOLDER: &str = "–––";
const METER_GAP: f32 = 8.0;
const TEXT_GAP: f32 = 12.0;

pub(super) struct DrawnLane {
    pub(super) text: Placed,
    pub(super) rate: Placed,
    pub(super) spark: Vec<Rectangle>,
}

pub(super) fn lane(
    agent: &Agent,
    (indent, row_top): (u16, f32),
    (stage, layout): (&Stage, &Layout),
    now: Instant,
    translator: &impl Translator,
) -> DrawnLane {
    let [left, right] = stage.column;
    let measures = agent.measures(now);
    let rate = measures
        .rate
        .map_or_else(|| PLACEHOLDER.to_owned(), |rate| rate.to_string());
    let rate = format!("{rate} {}", translator.translate(Text::UnitTokensPerSecond));
    let small = layout.advance_of(TextSize::Small);
    let room_in_small = index(f64::from((right - left) / small));
    let rate_indent = u16::try_from(room_in_small)
        .unwrap_or(u16::MAX)
        .saturating_sub(columns_of(&rate));
    let rate_left = left + f32::from(rate_indent) * small;
    let spark_width = stage.exact_points(MINI_SPARK_WIDTH);
    let spark_left = (rate_left - stage.points(METER_GAP) - spark_width).round();
    let spark_top = (row_top + (stage.row - stage.points(MINI_SPARK_HEIGHT)) / 2.0).round();

    let room = (spark_left - stage.points(TEXT_GAP) - left) / layout.advance_of(TextSize::Regular);
    let room = index(f64::from(room.max(0.0))).saturating_sub(usize::from(indent));
    let text = truncated(&told(agent), room);
    DrawnLane {
        text: Placed {
            top: layout.text_top(TextSize::Regular, row_top, stage.row),
            line: Line::new(indent, vec![Span::new(text, Ink::Full)]),
            clip: None,
        },
        rate: Placed {
            top: layout.text_top(TextSize::Small, row_top, stage.row),
            line: Line::new(rate_indent, vec![Span::new(rate, Ink::Live)]),
            clip: None,
        },
        spark: mini_spark(
            &measures.spark.rates,
            [spark_left, spark_top],
            stage,
            Some(Ink::Live),
        ),
    }
}

fn told(agent: &Agent) -> String {
    let name = format!("{:<NAME_COLUMNS$}", name_of(agent));
    match agent.current() {
        Some(action) => format!(
            "{RULE} {name} {ENTER} {} {}",
            action.tool.glyph.mark(),
            action.tool.label
        ),
        None => format!("{RULE} {name} {ENTER}"),
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/running/lane.rs"]
mod tests;
