use std::time::Instant;

use dot_api::Session;

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Bounds, Canvas, Layout, Scene};
use crate::conversation::Stage;
use crate::sessions::Rail;
use dot_transcript::to_f32;

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn compose(&mut self, now: Instant) {
        let layout = self.layout();
        let reticle = self.reticle();
        let (mut scene, caret) = match self.screen {
            Screen::SignIn => {
                self.wake_at = None;
                let lines = self.sign_in.lines(self.translator);
                let scene = Scene {
                    lines,
                    ..Scene::default()
                };
                (scene, None)
            }
            Screen::Chat => {
                let (scene, caret) = self.chat_scene(&layout, now);
                (scene, Some(caret))
            }
        };
        scene.layout = layout;
        scene.draw_pictograms();
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.show(scene);
            canvas.point(reticle);
            canvas.allow_composition(caret);
        }
    }

    pub(super) fn rail(&self, layout: &Layout, now: Instant) -> Option<Rail> {
        let stage = Stage::of(layout);
        let cards = self
            .sessions
            .cards(self.status_bar.project(), now, self.translator);
        let hover = self
            .window
            .cursor
            .map(|cursor| [to_f32(cursor.x), to_f32(cursor.y)]);
        Rail::of(&cards, (&stage, layout), hover, now, self.translator)
    }

    fn chat_scene(&mut self, layout: &Layout, now: Instant) -> (Scene, Bounds) {
        if let Some(editing) = self.editor_scene(layout, now) {
            return editing;
        }
        let focused = self.window.focused;
        let translator = self.translator;
        let active = self.sessions.active_mut();
        let conversation = &mut active.conversation;
        let pointer = self
            .window
            .cursor
            .map(|cursor| [to_f32(cursor.x), to_f32(cursor.y)]);
        conversation.point(pointer);
        conversation.prepare(layout, now, focused);
        let mut scene = conversation.scene(layout, now, translator);
        let mut activity = conversation.activity(layout, now, translator);
        let caret = conversation.caret_area(layout, now);
        let mut wake_at = conversation.next_change(now);
        let spending = active.session.spending();

        self.fade_back(&mut scene, Stage::of(layout).middle(), now);
        if let Some(switching) = self.sessions.switching(now) {
            scene.fade_around(Stage::of(layout).middle(), switching.alpha, switching.scale);
            scene.animating = true;
        }
        match self.rail(layout, now) {
            Some(rail) => {
                scene.panes.extend(rail.panes);
                scene.marks.extend(rail.marks);
                let blink = self.sessions.next_blink(now).filter(|_| focused);
                wake_at = wake_at.into_iter().chain(blink).min();
            }
            None => activity.sessions = Some(self.sessions.count()),
        }
        self.wake_at = wake_at;
        scene.strip = self
            .status_bar
            .strip(self.translator, layout, &activity, spending);
        (scene, caret)
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/frame.rs"]
mod tests;
