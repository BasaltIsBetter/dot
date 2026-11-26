use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::{Layout, Paint, Rectangle, Silhouette, TextSize};
use crate::conversation::flow::reply::name_of;
use crate::conversation::flow::running::columns_of;
use crate::conversation::stage::Stage;
use crate::conversation::{MINI_SPARK_HEIGHT, MINI_SPARK_WIDTH, mini_spark};
use dot_transcript::{Agent, Glyph, Ink, Line, Placed, Span, columns, index, truncated};

const FIGURE_COLUMNS: u16 = 3;
const NAME_COLUMNS: usize = 8;
const ENTER: char = '↗';
const PLACEHOLDER: &str = "–––";
const METER_GAP: f32 = 8.0;
const TEXT_GAP: f32 = 12.0;

pub(super) struct DrawnLane {
    pub(super) text: Placed,
    pub(super) rate: Placed,
    pub(super) spark: Vec<Rectangle>,
    pub(super) figure: Vec<Rectangle>,
}

pub(super) fn lane(
    agent: &Agent,
    (indent, row_top): (u16, f32),
    (stage, layout): (&Stage, &Layout),
    (silhouette, tool): (&Silhouette, Option<[u8; 3]>),
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

    let advance = layout.advance_of(TextSize::Regular);
    let text_indent = indent.saturating_add(FIGURE_COLUMNS);
    let room = (spark_left - stage.points(TEXT_GAP) - left) / advance;
    let room = index(f64::from(room.max(0.0))).saturating_sub(usize::from(text_indent));
    let figure_left = left + f32::from(indent) * advance;
    let figure_top = row_top + (stage.row - silhouette.size()[1]) / 2.0;
    let figure_ink = tool.map_or(Ink::Live, Ink::Rgb);
    DrawnLane {
        text: Placed {
            top: layout.text_top(TextSize::Regular, row_top, stage.row),
            line: Line::new(text_indent, told(agent, room, tool)),
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
        figure: silhouette
            .drawn([figure_left, figure_top], Paint::ink(figure_ink))
            .collect(),
    }
}

fn told(agent: &Agent, room: usize, tool: Option<[u8; 3]>) -> Vec<Span> {
    let named = format!("{:<NAME_COLUMNS$} {ENTER}", name_of(agent));
    let Some(action) = agent.current() else {
        return vec![Span::new(truncated(&named, room), Ink::Full)];
    };
    let glyph = action.tool.glyph;
    let text = truncated(
        &format!("{named} {} {}", glyph.mark(), action.tool.label),
        room,
    );
    let Some(tool) = tool.filter(|_| glyph != Glyph::Other) else {
        return vec![Span::new(text, Ink::Full)];
    };
    let mut characters = text.chars();
    let head: String = characters.by_ref().take(columns(&named) + 1).collect();
    let mut spans = vec![Span::new(head, Ink::Full)];
    let Some(shown) = characters.next() else {
        return spans;
    };
    let ink = if shown == glyph.mark() {
        Ink::Rgb(tool)
    } else {
        Ink::Full
    };
    spans.push(Span::new(shown.to_string(), ink));
    let rest: String = characters.collect();
    if !rest.is_empty() {
        spans.push(Span::new(rest, Ink::Full));
    }
    spans
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/running/lane.rs"]
mod tests;
