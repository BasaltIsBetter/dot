use std::time::Instant;

use dot_api::{Text, Translator};

use crate::canvas::{Bounds, Glide, Layout, Pane, Placed, SETTLE};
use crate::conversation::flow::Conversation;
use crate::conversation::zone::{Zones, centring};

impl Conversation {
    pub(crate) fn prepare(&mut self, layout: &Layout, now: Instant, translator: &impl Translator) {
        let refitted = self.fit(layout);
        if self.question.is_some() {
            return;
        }
        let content = self.prompter.single_row_columns(&self.hint(translator));
        let target = centring(layout, content);

        match self.glide.as_mut() {
            Some(glide) if !refitted => glide.toward(target, now),
            _ => self.glide = Some(Glide::at_rest(target, SETTLE)),
        }
    }

    pub(crate) fn caret_area(&self, layout: &Layout) -> Bounds {
        let zones = self.zones(layout);
        let caret = self
            .prompter
            .caret_area(zones.prompter, zones.prompter_rows, layout);
        Bounds {
            left: caret.left + self.settled_shift(layout),
            ..caret
        }
    }

    pub(super) fn prompter_pane(
        &self,
        zones: Zones,
        layout: &Layout,
        now: Instant,
        focused: bool,
        translator: &impl Translator,
    ) -> Pane {
        let shift = match self.question {
            Some(_) => 0.0,
            None => self.glide.map_or(0.0, |glide| glide.position(now)),
        };
        Pane {
            top: zones.prompter.top,
            bottom: zones.prompter.bottom,
            shift: pixels(shift, layout),
            lines: self.prompter_lines(zones, layout, focused, translator),
        }
    }

    pub(super) fn settled_shift(&self, layout: &Layout) -> f32 {
        self.glide
            .map_or(0.0, |glide| pixels(glide.target(), layout))
    }

    fn prompter_lines(
        &self,
        zones: Zones,
        layout: &Layout,
        focused: bool,
        translator: &impl Translator,
    ) -> Vec<Placed> {
        if let Some(question) = &self.question {
            return question.lines(zones.prompter, zones.prompter_rows, layout.line_height());
        }
        let placeholder = self.hint(translator);
        self.prompter.lines(
            zones.prompter,
            zones.prompter_rows,
            layout,
            focused,
            &placeholder,
        )
    }

    fn hint(&self, translator: &impl Translator) -> String {
        let hint = if self.transcript.is_live() {
            Text::PrompterReplying
        } else {
            Text::PrompterPlaceholder
        };
        translator.translate(hint).into_owned()
    }
}

fn pixels(columns: f32, layout: &Layout) -> f32 {
    (columns * layout.advance()).round()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/conversation/flow/pane.rs"]
mod tests;
