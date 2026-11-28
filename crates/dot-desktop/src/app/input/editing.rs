use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dot_api::{Session, Text, Translator, Workspace};
use dot_transcript::{Motion as Glide, opacity, to_f32};

use crate::app::{DesktopApp, Parts, Screen};
use crate::canvas::{Bounds, Layout, Scene};
use crate::conversation::Blink;
use crate::editor::{
    Disk, DiskNews, Editor, Gesture, LONGEST_EDITED, Pressed, Saving, activity, gesture,
};
use crate::event::{Keystroke, Wheel, WindowAction};
use crate::shell::{copy_text, pasted_text};
use crate::status_bar::Activity;
use crate::worker::Mailbox;

const BACK: Duration = Duration::from_millis(450);
const BACK_FROM: f64 = 1.03;
const ROWS_PER_WHEEL_LINE: f64 = 3.0;

pub(crate) struct Editing<W, M> {
    disk: Disk<W, M>,
    editor: Option<Editor>,
    opening: Option<String>,
    blink: Blink,
    left: Option<Instant>,
}

impl<W: Workspace + 'static, M: Mailbox<DiskNews>> Editing<W, M> {
    pub(crate) fn new(workspace: Arc<W>, mailbox: M) -> Self {
        Self {
            disk: Disk::new(workspace, mailbox),
            editor: None,
            opening: None,
            blink: Blink::default(),
            left: None,
        }
    }

    pub(crate) fn finish(&mut self) {
        self.disk.finish();
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.editor.is_some()
    }

    pub(crate) fn read_image(&mut self, path: PathBuf) {
        self.disk.read_image(path);
    }
}

impl<P: Parts> DesktopApp<'_, P> {
    pub(in crate::app) fn open_file(&mut self, path: String) {
        let path = self.in_session_folder(path);
        self.sessions.active_mut().conversation.clear_selection();
        self.editing.opening = Some(path.clone());
        self.editing.disk.open(path);
    }

    pub(in crate::app) fn disk_news(&mut self, news: DiskNews, now: Instant) {
        if let DiskNews::Opened { path, .. } = &news
            && self
                .editing
                .opening
                .take_if(|opening| opening == path)
                .is_none()
        {
            return;
        }
        match news {
            DiskNews::Opened {
                path,
                text: Ok(text),
                entries,
            } => {
                let touched = self.sessions.active().conversation.touched();
                self.editing.editor = Some(Editor::new(path, text, touched, entries, now));
                self.editing.blink.restart(now);
                self.editing.left = None;
            }
            DiskNews::Opened {
                text: Err(error), ..
            } => {
                tracing::warn!(%error, "a file could not be opened in the editor");
                let notice = self.translator.translate(error.text());
                self.sessions.active_mut().conversation.notice(&notice, now);
            }
            DiskNews::Saved { path, result } => {
                let editor = self.editing.editor.as_mut();
                match editor.filter(|editor| editor.path() == path) {
                    Some(editor) => editor.saved(result),
                    None if result.is_err() => self.tell_lost(&path, now),
                    None => {}
                }
            }
            DiskNews::Dropped { name, image } => self.attach(&name, image, now),
        }
    }

    pub(in crate::app) fn edit_input(&mut self, action: &WindowAction, now: Instant) -> bool {
        if self.screen != Screen::Chat || self.editing.editor.is_none() {
            return false;
        }
        match action {
            WindowAction::Key(keystroke) => self.edit_key(keystroke, now),
            WindowAction::Commit(text) => self.edit(Gesture::Insert(text.clone()), now),
            WindowAction::Press => {
                self.window.pressing = true;
                self.edit_press(now);
            }
            WindowAction::Point(position) => {
                self.window.cursor = Some(*position);
                if self.window.pressing {
                    self.edit_drag(now);
                }
            }
            WindowAction::Release => {
                self.window.pressing = false;
                if let Some(editor) = self.editing.editor.as_mut() {
                    editor.release();
                }
            }
            WindowAction::Scroll(wheel) => {
                let rows = match *wheel {
                    Wheel::Lines(lines) => f64::from(lines) * ROWS_PER_WHEEL_LINE,
                    Wheel::Pixels(pixels) => pixels / f64::from(self.layout(now).line_height()),
                };
                if let Some(editor) = self.editing.editor.as_mut() {
                    editor.scroll(-rows);
                }
            }
            _ => return false,
        }
        true
    }

    pub(in crate::app) fn editor_scene(
        &mut self,
        layout: &Layout,
        now: Instant,
    ) -> Option<(Scene, Bounds)> {
        let focused = self.window.focused;
        let pointer = self
            .window
            .cursor
            .map(|cursor| [to_f32(cursor.x), to_f32(cursor.y)]);
        self.editing.blink.pace(focused, now);
        let caret = self.editing.blink.caret(now);
        let translator = self.translator;
        let pressing = self.window.pressing;
        let editor = self.editing.editor.as_mut()?;
        let at_an_edge = pointer
            .filter(|_| pressing)
            .is_some_and(|at| editor.follow_edges(at, layout));
        let mut scene = editor.scene(layout, now, (pointer, caret), translator);
        scene.animating |= at_an_edge;
        let bounds = editor.caret_area(layout);
        let mut activity = Activity {
            text: Some(activity(editor, translator)),
            live: false,
            sessions: Some(self.sessions.count()),
            model_heat: 0,
            effort_heat: 0,
            gauge_tint: None,
            gauge_fill: None,
            name: None,
            name_tint: None,
        };
        let blink = self.editing.blink.next_change(now).filter(|_| focused);
        let opening = editor.is_opening(now).then_some(now);
        let pacing = self.pacing_wake(now);
        self.wake_at = blink.into_iter().chain(opening).chain(pacing).min();
        let spending = self.sessions.active().session.spending();
        self.dress(&mut activity, spending.usage.context_tokens, now);
        let tuning = self.sessions.tuning();
        scene.strip = self
            .status_bar
            .strip(translator, layout, (&activity, &tuning), spending);
        Some((scene, bounds))
    }

    pub(in crate::app) fn fade_back(&self, scene: &mut Scene, middle: [f32; 2], now: Instant) {
        let Some(left) = self.editing.left else {
            return;
        };
        let Some(share) = Glide::landing(BACK).eased(now.saturating_duration_since(left)) else {
            return;
        };
        scene.fade_around(
            middle,
            opacity(share),
            BACK_FROM - (BACK_FROM - 1.0) * share,
        );
        scene.animating = true;
    }

    fn edit_key(&mut self, keystroke: &Keystroke, now: Instant) {
        let Keystroke {
            key,
            text,
            modifiers,
        } = keystroke;
        if let Some(gesture) = gesture(key, text.as_deref(), *modifiers) {
            self.edit(gesture, now);
        }
    }

    fn edit(&mut self, gesture: Gesture, now: Instant) {
        self.editing.blink.restart(now);
        let Some(editor) = self.editing.editor.as_mut() else {
            return;
        };
        let acted = editor.act(gesture, now, || pasted_text(LONGEST_EDITED));
        if let Some(copied) = &acted.copied {
            copy_text(copied);
        }
        if acted.changed {
            self.editing.disk.save(editor.path(), editor.text());
        }
        if acted.closed {
            self.close_editor(now);
        }
    }

    fn edit_press(&mut self, now: Instant) {
        let Some(cursor) = self.window.cursor else {
            return;
        };
        let layout = self.layout(now);
        let extend = self.window.modifiers.shift_key();
        let translator = self.translator;
        let clicks = self.count_click([to_f32(cursor.x), to_f32(cursor.y)], now);
        self.editing.blink.restart(now);
        let Some(editor) = self.editing.editor.as_mut() else {
            return;
        };
        let at = [to_f32(cursor.x), to_f32(cursor.y)];
        match editor.press(at, (extend, clicks, now), &layout, translator) {
            Pressed::Changed => self.editing.disk.save(editor.path(), editor.text()),
            Pressed::Open(path) => self.open_file(path),
            Pressed::Close => self.close_editor(now),
            Pressed::Placed | Pressed::Nothing => {}
        }
    }

    fn edit_drag(&mut self, now: Instant) {
        let Some(cursor) = self.window.cursor else {
            return;
        };
        let layout = self.layout(now);
        self.editing.blink.restart(now);
        if let Some(editor) = self.editing.editor.as_mut() {
            editor.drag([to_f32(cursor.x), to_f32(cursor.y)], &layout);
        }
    }

    fn close_editor(&mut self, now: Instant) {
        let closed = self.editing.editor.take();
        self.editing.left = Some(now);
        if let Some(editor) = closed.filter(|editor| matches!(editor.saving(), Saving::Failed(_))) {
            self.tell_lost(editor.path(), now);
        }
    }

    fn tell_lost(&mut self, path: &str, now: Instant) {
        let notice = self.translator.with(Text::EditorLost, path);
        self.sessions.active_mut().conversation.notice(&notice, now);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/app/input/editing.rs"]
mod tests;
