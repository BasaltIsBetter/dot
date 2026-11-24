use dot_api::{Text, Translator};

use crate::canvas::{Paint, Pane, TextSize};
use crate::conversation::{MINI_SPARK_HEIGHT, mini_spark};
use crate::sessions::card::{Card, State};
use crate::sessions::rail::mark::{corners, glyph_of, plus};
use crate::sessions::rail::{
    CORNER, FOOT, Frame, HEAD, Hit, PADDING_WIDE, QUIET, Rail, to_columns, to_points,
};
use dot_transcript::{Ink, Line, Placed, Span, columns};

const END_GAP: f32 = 8.0;
const PLUS_ARM: f32 = 7.0;
const CLOSE: &str = "×";
const PLACEHOLDER: &str = "–––";

impl Rail {
    pub(super) fn head(
        &mut self,
        frame: &Frame<'_>,
        (index, card): (usize, &Card),
        top: f32,
        (closable, hovered): (bool, bool),
    ) {
        let stage = frame.stage;
        let small = frame.layout.advance_of(TextSize::Small);
        let [name_left, inner_right] = frame.inner();
        let close_left = inner_right - small;
        let glyph_left = if closable {
            close_left - stage.points(END_GAP) - small
        } else {
            inner_right - small
        };
        let name_left = self.square(frame, (index, card), [name_left, top]);
        let name_end = glyph_left - stage.points(END_GAP);
        let name_room = ((name_end - name_left) / small).max(0.0);
        self.name(
            frame,
            (index, card),
            [name_left, top, name_end - name_left],
            (to_columns(name_room), hovered),
        );
        let (glyph, glyph_ink) = glyph_of(card);
        self.text(
            frame,
            [glyph_left, top],
            TextSize::Small,
            Span::new(glyph, glyph_ink),
        );
        if !(closable && hovered) {
            return;
        }

        let close = [close_left, top, small, stage.points(HEAD)];
        let ink = if frame.is_over(close) {
            Ink::Full
        } else {
            Ink::Receding
        };
        self.text(
            frame,
            [close_left, top],
            TextSize::Small,
            Span::new(CLOSE, ink),
        );
        self.hits.push((close, Hit::Close(index)));
    }

    pub(super) fn foot(
        &mut self,
        frame: &Frame<'_>,
        card: &Card,
        top: f32,
        translator: &impl Translator,
    ) {
        let stage = frame.stage;
        let [name_left, inner_right] = frame.inner();
        let live = card.state == State::Writing;
        let live_ink = card.hue.map_or(Ink::Live, |hue| Ink::Rgb(hue.rgb()));
        let spark_top =
            (top + (stage.points(FOOT) - stage.points(MINI_SPARK_HEIGHT)) / 2.0).round();
        self.marks.extend(mini_spark(
            &card.foot.rates,
            [name_left, spark_top],
            stage,
            live.then_some(live_ink),
        ));

        let per_second = translator.translate(Text::UnitTokensPerSecond);
        let rate = card
            .foot
            .rate
            .map_or_else(|| PLACEHOLDER.to_owned(), |rate| rate.to_string());
        let text = format!("{rate} {per_second}");
        let tiny = frame.layout.advance_of(TextSize::Tiny);
        let rate_left = inner_right - to_points(columns(&text)) * tiny;
        let ink = if live { live_ink } else { QUIET };
        self.text(
            frame,
            [rate_left, top],
            TextSize::Tiny,
            Span::new(text, ink),
        );
    }

    pub(super) fn new_card(&mut self, frame: &Frame<'_>, [top, height]: [f32; 2]) {
        let bounds = [frame.left, top, frame.right - frame.left, height];
        let hovered = frame.is_over(bounds);
        self.hits.push((bounds, Hit::Open));
        let colour = Paint::ink(if hovered { Ink::Full } else { Ink::Receding });
        self.marks
            .extend(corners(bounds, frame.stage.points(CORNER), colour));
        let stage = frame.stage;
        let arm = stage.points(PLUS_ARM);
        let centre = [
            frame.left + stage.points(PADDING_WIDE) + arm / 2.0,
            top + height / 2.0,
        ];
        let colour = Paint::ink(if hovered { Ink::Full } else { QUIET });
        self.marks.extend(plus(centre, arm, colour));
    }

    pub(super) fn text(
        &mut self,
        frame: &Frame<'_>,
        [left, top]: [f32; 2],
        size: TextSize,
        span: Span,
    ) {
        let row = match size {
            TextSize::Tiny => frame.stage.points(FOOT),
            _ => frame.stage.points(HEAD),
        };
        self.panes.push(Pane {
            left,
            size,
            top: 0.0,
            bottom: frame.stage.floor,
            lines: vec![Placed {
                top: frame.layout.text_top(size, top, row),
                line: Line::new(0, vec![span]),
                clip: None,
            }],
        });
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/sessions/rail/face.rs"]
mod tests;
