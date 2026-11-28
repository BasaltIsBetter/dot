use std::time::{Duration, Instant};

use dot_api::{CaretBlink, Session};

use crate::app::settings::settings_frame;
use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Bounds, Canvas, Layout, Scene};
use crate::conversation::Stage;
use crate::sessions::{CardLook, Rail};
use crate::status_bar::{Activity, StatusBar};
use dot_transcript::{Ink, OPAQUE, opacity, to_f32};

const PACING_STEP: Duration = Duration::from_millis(100);

impl<P: Parts> DesktopApp<'_, P> {
    pub(super) fn compose(&mut self, now: Instant) {
        self.flow_replies(now);
        let layout = self.layout(now);
        let reticle = self.reticle(&layout, now);
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
        if let (Some(strip), Some(pointer)) = (scene.strip.as_mut(), self.pointer()) {
            strip.light_under(&layout, pointer);
        }
        scene.layout = layout;
        scene.palette = self.palette();
        self.strip.clone_from(&scene.strip);
        scene.draw_pictograms();
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.show(scene);
            canvas.point(reticle);
            canvas.allow_composition(caret);
        }
    }

    pub(super) fn rail(&self, layout: &Layout, now: Instant) -> Option<Rail> {
        let stage = Stage::of(layout);
        let look = CardLook {
            project: self.status_bar.project(),
            renaming: self.renaming.as_ref(),
        };
        let cards = self.sessions.cards(&look, now, self.translator);
        let hover = self
            .window
            .cursor
            .map(|cursor| [to_f32(cursor.x), to_f32(cursor.y)]);
        let offset = self.sessions.rail_offset();
        Rail::of(
            &cards,
            (&stage, layout),
            (hover, offset),
            now,
            self.translator,
        )
    }

    pub(super) fn pacing_wake(&self, now: Instant) -> Option<Instant> {
        self.sessions.are_pacing().then_some(now + PACING_STEP)
    }

    fn flow_replies(&mut self, now: Instant) {
        let translator = self.translator;
        for (active, opened) in self.sessions.each_mut() {
            let Some(ended) = opened.conversation.flow(now, translator) else {
                continue;
            };
            if let Some(prompt) = ended.next {
                opened.ask(prompt, &mut self.recall);
            }
            if !active && !opened.conversation.is_live() {
                opened.unread = Some(now);
            }
        }
    }

    pub(super) fn dress(&mut self, activity: &mut Activity, context_tokens: u64, now: Instant) {
        let active = self.sessions.active();
        activity.name.clone_from(&active.identity.name);
        let heat = active.identity.name_heat(now);
        activity.name_tint = (heat > 0).then(|| self.palette().shade(Ink::Full, heat, OPAQUE));
        let window = self.sessions.tuning().model.context_window;
        let colour = self
            .settings
            .state_colors
            .is_on()
            .then(|| self.sessions.hue().rgb());
        let context = (context_tokens, window);
        self.status_bar.gauge_for(active.key);
        activity.gauge_tint = self.status_bar.gauge_tint(context, colour, now);
        activity.gauge_fill = Some(self.status_bar.gauge_fill(context, now));
    }

    fn chat_scene(&mut self, layout: &Layout, now: Instant) -> (Scene, Bounds) {
        if let Some(editing) = self.editor_scene(layout, now) {
            return editing;
        }
        let focused = self.window.focused;
        let translator = self.translator;
        let pointer = self.pointer();
        let steady = self.settings.caret == CaretBlink::Steady;
        let colours = self.colours();
        let active = self.sessions.active_mut();
        let conversation = &mut active.conversation;
        conversation.point(pointer);
        conversation.hold_caret(steady);
        conversation.colour(colours);
        conversation.prepare(layout, now, focused);
        let mut scene = conversation.scene(layout, now, translator);
        let mut activity = conversation.activity(layout, now, translator);
        let caret = conversation.caret_area(layout, now);
        let mut wake_at = conversation.next_change(now);
        let spending = active.session.spending();
        self.dress(&mut activity, spending.usage.context_tokens, now);

        self.fade_back(&mut scene, Stage::of(layout).middle(), now);
        self.sessions.follow_rail(&Stage::of(layout));
        match self.rail(layout, now) {
            Some(rail) => {
                scene.panes.extend(rail.panes);
                scene.marks.extend(rail.marks);
                let blink = self.sessions.next_blink(now).filter(|_| focused);
                wake_at = wake_at.into_iter().chain(blink).min();
            }
            None => activity.sessions = Some(self.sessions.count()),
        }
        self.wake_at = wake_at.into_iter().chain(self.pacing_wake(now)).min();
        self.settings_over(&mut scene, layout, now);
        [activity.model_heat, activity.effort_heat] = self.tuned_heats(now);
        scene.strip = if self.settings_screen.is_open() {
            Some(StatusBar::<P::Mailbox>::searching(
                translator,
                layout,
                &self.searching(now),
            ))
        } else {
            let tuning = self.sessions.tuning();
            self.status_bar
                .strip(translator, layout, (&activity, &tuning), spending)
        };
        scene.animating |= self.settings_are_moving(now)
            || self.status_bar.is_cooling(now)
            || self.sessions.names_are_cooling(now);
        (scene, caret)
    }

    fn settings_over(&self, scene: &mut Scene, layout: &Layout, now: Instant) {
        let shown = self.settings_screen.shown(now);
        if shown <= 0.0 {
            return;
        }
        if shown >= 1.0 {
            scene.panes.clear();
            scene.grounds.clear();
            scene.marks.clear();
        } else {
            scene.fade_around(Stage::of(layout).middle(), opacity(1.0 - shown), 1.0);
        }
        let defaults = *self.keeper.defaults();
        let actions = self.actions();
        let look = self.settings_look((&defaults, &actions));
        let frame = settings_frame(layout);
        self.settings_screen
            .draw(scene, &look, (layout, &frame), now);
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/frame.rs"]
mod tests;
