use std::time::Instant;

use dot_api::{Text, Translator};
use winit::dpi::PhysicalPosition;

use crate::canvas::{Bounds, Ink, Layout, Pane, Rectangle, Scene, to_f32};
use crate::conversation::prompter::{Edit, Intent, Prompter};
use crate::conversation::transcript::{Transcript, lines};
use crate::conversation::zone::{Area, Zones};
use crate::event::Wheel;

const ROWS_PER_WHEEL_LINE: f64 = 3.0;
const RULE_INK: Ink = Ink::Faint;

#[derive(Debug, Default)]
pub(crate) struct Conversation {
    prompter: Prompter,
    transcript: Transcript,
}

impl Conversation {
    pub(crate) fn type_key(
        &mut self,
        intent: Intent,
        layout: &Layout,
        now: Instant,
        clipboard: impl FnOnce() -> Option<String>,
    ) {
        match intent {
            Intent::Edit(edit) => self.edit(edit, layout),
            Intent::Send => self.send(now),
            Intent::Paste => {
                if let Some(text) = clipboard() {
                    self.edit(Edit::Insert(text), layout);
                }
            }
        }
    }

    pub(crate) fn commit(&mut self, text: String, layout: &Layout) {
        self.edit(Edit::Insert(text), layout);
    }

    pub(crate) fn compose(&mut self, text: String) {
        self.prompter.compose(text);
    }

    pub(crate) fn scroll(&mut self, wheel: Wheel, at: PhysicalPosition<f64>, layout: &Layout) {
        let zones = self.zones(layout);
        let line_height = layout.line_height();
        let rows_up = match wheel {
            Wheel::Lines(lines) => f64::from(lines) * ROWS_PER_WHEEL_LINE,
            Wheel::Pixels(pixels) => pixels / f64::from(line_height),
        };

        match zones.area_at(to_f32(at.y)) {
            Some(Area::Transcript) => self
                .transcript
                .scroll_by(rows_up, zones.transcript.rows(line_height)),
            Some(Area::Prompter) => self.prompter.scroll_by(rows_up, zones.prompter_rows),
            None => {}
        }
    }

    pub(crate) fn press(&mut self, at: PhysicalPosition<f64>, layout: &Layout) {
        let zones = self.zones(layout);
        let point = [to_f32(at.x), to_f32(at.y)];
        if zones.area_at(point[1]) == Some(Area::Prompter) {
            self.prompter
                .place(point, zones.prompter, zones.prompter_rows, layout);
        }
    }

    pub(crate) fn scene(
        &self,
        layout: &Layout,
        now: Instant,
        focused: bool,
        translator: &impl Translator,
    ) -> Scene {
        let zones = self.zones(layout);
        let placeholder = translator.translate(Text::PrompterPlaceholder);
        let transcript = Pane {
            top: zones.transcript.top,
            bottom: zones.transcript.bottom,
            lines: lines(&self.transcript, zones.transcript, layout, now),
        };
        let prompter = Pane {
            top: zones.prompter.top,
            bottom: zones.prompter.bottom,
            lines: self.prompter.lines(
                zones.prompter,
                zones.prompter_rows,
                layout,
                focused,
                &placeholder,
            ),
        };
        let left = layout.x(0).round();
        let rule = Rectangle {
            left,
            top: zones.rule,
            width: layout.x(Layout::COLUMNS).round() - left,
            height: Layout::HAIRLINE,
            ink: RULE_INK,
        };

        Scene {
            panes: vec![transcript, prompter],
            rules: vec![rule],
            animating: self.transcript.is_fresh(now),
            ..Scene::default()
        }
    }

    pub(crate) fn caret_area(&self, layout: &Layout) -> Bounds {
        let zones = self.zones(layout);
        self.prompter
            .caret_area(zones.prompter, zones.prompter_rows, layout)
    }

    fn edit(&mut self, edit: Edit, layout: &Layout) {
        self.prompter.edit(edit);
        let zones = self.zones(layout);
        self.prompter.reveal_caret(zones.prompter_rows);
        self.transcript
            .refit(zones.transcript.rows(layout.line_height()));
    }

    fn send(&mut self, now: Instant) {
        if let Some(text) = self.prompter.take() {
            self.transcript.append(text, now);
        }
    }

    fn zones(&self, layout: &Layout) -> Zones {
        Zones::new(layout, self.prompter.rows())
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/conversation/flow.rs"]
mod tests;
