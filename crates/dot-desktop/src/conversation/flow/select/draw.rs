use std::time::Instant;

use dot_api::Translator;
use dot_transcript::{Ink, OPAQUE, Selection, TEXT_INDENT, columns, opacity, rows, to_f32};

use crate::canvas::{Layout, Paint, Pane, Rectangle};
use crate::conversation::Conversation;
use crate::conversation::flow::select::outline::{Marks, Outline, RowSpan};
use crate::conversation::flow::select::{COOLING, SETTLING};
use crate::conversation::stage::Stage;

const WASH_MAKING: f64 = 0.38;
const WASH_MADE: f64 = 0.21;
const HAIRLINE: f32 = Layout::HAIRLINE;
const BRACKET: f32 = 6.0;
const BEAM_INSET: f32 = 2.0;
const BEAM_SERIF: f32 = 2.5;
const NEW_LINE_STUB: f32 = 0.5;
const BEAM_INK: Ink = Ink::Secondary;
const MADE_MARK_INK: Ink = Ink::Secondary;

#[derive(Debug, Default)]
pub(crate) struct Drawn {
    pub(crate) grounds: Vec<Rectangle>,
    pub(crate) marks: Vec<Rectangle>,
    pub(crate) panes: Vec<Pane>,
}

#[derive(Clone, Copy)]
struct Reach {
    indent: u16,
    from: usize,
    to: usize,
    stub: f32,
}

impl Conversation {
    pub(crate) fn selection_drawn(
        &self,
        (stage, layout): (&Stage, &Layout),
        top_row: f64,
        now: Instant,
        translator: &impl Translator,
    ) -> Drawn {
        let mut drawn = Drawn::default();
        let Some(selecting) = self.selecting.as_ref() else {
            self.hover_beam(&mut drawn, (stage, layout), now);
            return drawn;
        };
        let place = (stage, layout, top_row);
        let outline = self.outline_of(&selecting.selection, place);
        let arm = stage.points(BRACKET);
        let Some(made) = selecting.made else {
            let live_wash = Paint::shaded(Ink::Live, 0, opacity(WASH_MAKING));
            drawn.grounds.extend(outline.wash(live_wash));
            let words = selecting.selection.to_words(&self.transcript);
            let marks = Marks {
                color: Paint::ink(Ink::Live),
                wash: WASH_MAKING,
                arm,
            };
            self.outline_of(&words, place)
                .frame(marks, &mut drawn.marks);
            let (start, end) = selecting.selection.ends();
            for spot in [start, end] {
                if let Some(x) = self.x_of(spot.row, spot.column, (stage, layout)) {
                    let top = stage.row_top(spot.row, top_row);
                    drawn.marks.extend(beam([x, top], stage, Ink::Live));
                }
            }
            return clipped(drawn, stage);
        };
        let settled = SETTLING
            .eased(now.saturating_duration_since(made))
            .unwrap_or(1.0);
        let wash = settled_share(settled);
        drawn.grounds.extend(outline.wash(Paint::shaded(
            Ink::Full,
            opacity(1.0 - settled),
            opacity(wash),
        )));
        let since = selecting.copied.map_or(made, |copied| copied.max(made));
        let marks = Marks {
            color: Paint::shaded(MADE_MARK_INK, cooling_heat(since, now), OPAQUE),
            wash: wash / level_of(MADE_MARK_INK),
            arm,
        };
        outline.frame(marks, &mut drawn.marks);
        self.hover_beam(&mut drawn, (stage, layout), now);
        if stage.gutter.is_some() {
            let (start, _) = selecting.selection.ends();
            let top = stage
                .row_top(start.row, top_row)
                .clamp(0.0, stage.floor - stage.row);
            let tag = self.selection_label(selecting.copied, (stage, layout), top, now, translator);
            drawn.panes.push(tag);
        }
        clipped(drawn, stage)
    }

    fn outline_of(
        &self,
        selection: &Selection,
        (stage, layout, top_row): (&Stage, &Layout, f64),
    ) -> Outline {
        let (start, end) = selection.ends();
        let closing_row = match self.reach_on(selection, end.row) {
            Some(_) => end.row,
            None => end.row.saturating_sub(1),
        };
        let mut outline = Outline::default();
        for row in start.row..=closing_row {
            let top = stage.row_top(row, top_row);
            let bottom = stage.row_top(row + 1, top_row);
            if bottom <= 0.0 || top >= stage.floor {
                continue;
            }
            let Some(reach) = self.reach_on(selection, row) else {
                continue;
            };
            if outline.spans.is_empty() {
                outline.opens = row == start.row;
            }
            outline.closes = row == closing_row;
            let x = |column: usize, stub: f32| {
                (column_x(reach.indent, column, (stage, layout)) + stub * layout.advance()).round()
            };
            outline.spans.push(RowSpan {
                left: x(reach.from, 0.0),
                right: x(reach.to, reach.stub),
                top,
                bottom,
            });
        }
        outline
    }

    fn reach_on(&self, selection: &Selection, row: usize) -> Option<Reach> {
        let (from, to) = selection.columns_on(row)?;
        let (_, end) = selection.ends();
        let (text, indent) = self
            .transcript
            .row_text(row)
            .map_or(("", TEXT_INDENT), |shown| (shown.text, shown.indent));
        let from = from.min(columns(text));
        let to = if row == end.row {
            to.min(columns(text))
        } else {
            columns(text.trim_end())
        };
        if to > from {
            return Some(Reach {
                indent,
                from,
                to,
                stub: 0.0,
            });
        }
        (row != end.row).then_some(Reach {
            indent,
            from,
            to: from,
            stub: NEW_LINE_STUB,
        })
    }

    fn hover_beam(&self, drawn: &mut Drawn, (stage, layout): (&Stage, &Layout), now: Instant) {
        let Some(pointer) = self.pointer else {
            return;
        };
        let Some(spot) = self.text_spot(pointer, layout, now) else {
            return;
        };
        let focus = self.reader.focus(now);
        if let Some(x) = self.x_of(spot.row, spot.column, (stage, layout)) {
            let top = stage.row_top(spot.row, stage.top_row(focus));
            drawn.marks.extend(beam([x, top], stage, BEAM_INK));
        }
    }

    fn x_of(&self, row: usize, column: usize, (stage, layout): (&Stage, &Layout)) -> Option<f32> {
        let shown = self.transcript.row_text(row)?;
        let column = column.min(columns(shown.text));
        Some(column_x(shown.indent, column, (stage, layout)).round())
    }
}

fn column_x(indent: u16, column: usize, (stage, layout): (&Stage, &Layout)) -> f32 {
    stage.column[0] + (f32::from(indent) + to_f32(rows(column))) * layout.advance()
}

fn settled_share(settled: f64) -> f64 {
    WASH_MAKING + (WASH_MADE - WASH_MAKING) * settled
}

fn level_of(ink: Ink) -> f64 {
    let [level, ..] = ink.rgb();
    f64::from(level) / f64::from(OPAQUE)
}

fn beam([x, top]: [f32; 2], stage: &Stage, ink: Ink) -> [Rectangle; 3] {
    let inset = stage.points(BEAM_INSET);
    let serif = stage.points(BEAM_SERIF);
    let top = top.round() + inset;
    let height = stage.row.round() - 2.0 * inset;
    let color = Paint::ink(ink);
    let serif_at = |top: f32| Rectangle {
        left: x - serif,
        top,
        width: 2.0 * serif + HAIRLINE,
        height: HAIRLINE,
        color,
    };
    [
        Rectangle {
            left: x,
            top,
            width: HAIRLINE,
            height,
            color,
        },
        serif_at(top),
        serif_at(top + height - HAIRLINE),
    ]
}

fn clipped(mut drawn: Drawn, stage: &Stage) -> Drawn {
    let band = [0.0, stage.floor];
    drawn.grounds.retain_mut(|ground| ground.clip_to(band));
    drawn.marks.retain_mut(|mark| mark.clip_to(band));
    drawn
}

pub(super) fn cooling_heat(since: Instant, now: Instant) -> u8 {
    COOLING
        .eased(now.saturating_duration_since(since))
        .map_or(0, |eased| opacity(1.0 - eased))
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/select/draw.rs"]
mod tests;
