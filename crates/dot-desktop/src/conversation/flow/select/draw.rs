use std::time::Instant;

use dot_api::{Text, Translator};
use dot_transcript::{Ink, Line, Placed, Span, columns, opacity, rows, to_f32};

use crate::canvas::{Layout, Paint, Pane, Rectangle, TextSize};
use crate::conversation::Conversation;
use crate::conversation::flow::select::{COOLING, COPIED_SHOWN, SETTLING};
use crate::conversation::stage::Stage;

const WASH_MAKING: u8 = 97;
const WASH_MADE: u8 = 54;
const HAIRLINE: f32 = 1.0;
const TICK: f32 = 3.0;
const BEAM_INK: Ink = Ink::Secondary;
const MADE_BRACKET_INK: Ink = Ink::Secondary;
const LABEL_INK: Ink = Ink::Secondary;
const VALUE_INK: Ink = Ink::Full;
const FROM_THE_COLUMN: f32 = 32.0;
const LABEL_GAP: &str = "  ";
const COPY_KEY: &str = "⌘C";

#[derive(Debug, Default)]
pub(crate) struct Drawn {
    pub(crate) grounds: Vec<Rectangle>,
    pub(crate) marks: Vec<Rectangle>,
    pub(crate) panes: Vec<Pane>,
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
        let advance = layout.advance();
        let Some(selecting) = self.selecting.as_ref() else {
            self.hover_beam(&mut drawn, (stage, layout), now);
            return drawn;
        };
        let (start, end) = selecting.selection.ends();
        let settled = selecting.made.map(|made| {
            SETTLING
                .eased(now.saturating_duration_since(made))
                .unwrap_or(1.0)
        });
        let wash = match settled {
            None => Paint {
                alpha: WASH_MAKING,
                ..Paint::ink(Ink::Live)
            },
            Some(eased) => Paint {
                alpha: blend(WASH_MAKING, WASH_MADE, eased),
                ..Paint::shaded(Ink::Full, opacity(1.0 - eased), u8::MAX)
            },
        };
        for row in start.row..=end.row {
            let Some((from, to)) = selecting.selection.columns_on(row) else {
                continue;
            };
            let Some(shown) = self.transcript.row_text(row) else {
                continue;
            };
            let top = stage.row_top(row, top_row);
            if top + stage.row < 0.0 || top > stage.floor {
                continue;
            }
            let left = stage.column[0] + f32::from(shown.indent) * advance;
            let length = shown.text.chars().count();
            let [from, to] = [from.min(length), to.min(length)];
            let width = to_f32(rows(to.saturating_sub(from))) * advance;
            drawn.grounds.push(Rectangle {
                left: (left + to_f32(rows(from)) * advance).round(),
                top: top.round(),
                width: width.round(),
                height: stage.row,
                color: wash,
            });
        }
        let bracket_paint = match selecting.made {
            None => Paint::ink(Ink::Live),
            Some(made) => {
                let since = selecting.copied.map_or(made, |copied| copied.max(made));
                Paint::shaded(MADE_BRACKET_INK, cooling_heat(since, now), u8::MAX)
            }
        };
        let words = selecting.selection.to_words(&self.transcript);
        let (word_start, word_end) = words.ends();
        for (spot, opens) in [(word_start, true), (word_end, false)] {
            if let Some(x) = self.x_of(spot.row, spot.column, (stage, layout)) {
                let top = stage.row_top(spot.row, top_row);
                drawn.marks.extend(bracket(
                    [x, top, stage.row],
                    opens,
                    stage.points(TICK),
                    bracket_paint,
                ));
            }
        }
        if settled.is_none() {
            for spot in [start, end] {
                if let Some(x) = self.x_of(spot.row, spot.column, (stage, layout)) {
                    let top = stage.row_top(spot.row, top_row);
                    drawn.marks.push(beam([x, top, stage.row], Ink::Live));
                }
            }
            return drawn;
        }
        let label_top = stage
            .row_top(start.row, top_row)
            .clamp(0.0, stage.floor - stage.row);
        drawn.panes.push(self.selection_label(
            selecting.copied,
            (stage, layout),
            label_top,
            now,
            translator,
        ));
        drawn
    }

    fn selection_label(
        &self,
        copied: Option<Instant>,
        (stage, layout): (&Stage, &Layout),
        top: f32,
        now: Instant,
        translator: &impl Translator,
    ) -> Pane {
        let (characters, lines) = self.selecting.as_ref().map_or((0, 0), |selecting| {
            selecting.selection.size(&self.transcript)
        });
        let copied = copied.filter(|at| COPIED_SHOWN.lasts(now.saturating_duration_since(*at)));
        let (copy, heat) = match copied {
            Some(at) => (
                translator.translate(Text::SelectionCopied),
                cooling_heat(at, now),
            ),
            None => (translator.translate(Text::SelectionCopy), 0),
        };
        let label = if lines > 1 {
            translator.with(Text::SelectionLines, &lines.to_string())
        } else {
            translator.translate(Text::SelectionLabel)
        };
        let count = translator.with(Text::SelectionCharacters, &characters.to_string());
        let labels = columns(&label).max(columns(COPY_KEY));
        let values = columns(&count).max(columns(&copy));
        let size_line = tag_line((&label, labels), Span::new(count.as_ref(), VALUE_INK));
        let copy_line = tag_line(
            (COPY_KEY, labels),
            Span::new(copy.as_ref(), VALUE_INK).heated(heat),
        );
        let small = layout.advance_of(TextSize::Small);
        let right = stage.column[0] - stage.points(FROM_THE_COLUMN);
        let widest = labels + columns(LABEL_GAP) + values;
        let left = right - to_f32(rows(widest)) * small;
        let line_height = layout.line_height_of(TextSize::Small);
        Pane {
            left: left.round(),
            size: TextSize::Small,
            top: 0.0,
            bottom: stage.floor,
            lines: vec![
                Placed {
                    top: layout.text_top(TextSize::Small, top, stage.row),
                    line: size_line,
                    clip: None,
                },
                Placed {
                    top: layout.text_top(TextSize::Small, top, stage.row) + line_height,
                    line: copy_line,
                    clip: None,
                },
            ],
        }
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
            drawn.marks.push(beam([x, top, stage.row], BEAM_INK));
        }
    }

    fn x_of(&self, row: usize, column: usize, (stage, layout): (&Stage, &Layout)) -> Option<f32> {
        let shown = self.transcript.row_text(row)?;
        let left = stage.column[0] + f32::from(shown.indent) * layout.advance();
        let column = column.min(shown.text.chars().count());
        Some((left + to_f32(rows(column)) * layout.advance()).round())
    }
}

fn beam([x, top, height]: [f32; 3], ink: Ink) -> Rectangle {
    Rectangle {
        left: x,
        top: top.round(),
        width: HAIRLINE,
        height,
        color: Paint::ink(ink),
    }
}

fn tag_line((label, labels): (&str, usize), value: Span) -> Line {
    let indent = labels.saturating_sub(columns(label));
    Line::new(
        u16::try_from(indent).unwrap_or(u16::MAX),
        vec![
            Span::new(label, LABEL_INK),
            Span::new(LABEL_GAP, LABEL_INK),
            value,
        ],
    )
}

fn cooling_heat(since: Instant, now: Instant) -> u8 {
    COOLING
        .eased(now.saturating_duration_since(since))
        .map_or(0, |eased| opacity(1.0 - eased))
}

fn bracket([x, top, height]: [f32; 3], opens: bool, tick: f32, colour: Paint) -> [Rectangle; 3] {
    let tick_left = if opens { x } else { x - tick + HAIRLINE };
    let top = top.round();
    [
        Rectangle {
            left: x,
            top,
            width: HAIRLINE,
            height,
            color: colour,
        },
        Rectangle {
            left: tick_left,
            top,
            width: tick,
            height: HAIRLINE,
            color: colour,
        },
        Rectangle {
            left: tick_left,
            top: top + height - HAIRLINE,
            width: tick,
            height: HAIRLINE,
            color: colour,
        },
    ]
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a blend of two alphas stays within 0 to 255"
)]
fn blend(from: u8, to: u8, share: f64) -> u8 {
    (f64::from(from) + (f64::from(to) - f64::from(from)) * share.clamp(0.0, 1.0)).round() as u8
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-desktop/conversation/flow/select/draw.rs"]
mod tests;
