mod buffer;
mod disk;
mod edit;
mod versions;
mod view;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use dot_api::{FileChange, Hunk, WorkspaceError};
use dot_transcript::{CodeLanguage, Motion as Glide, TextWidth, opacity};

use crate::canvas::Scene;
use crate::editor::buffer::Buffer;
use crate::editor::disk::folder_of;
use crate::editor::versions::{Lines, Versions};

pub(crate) use self::disk::{Disk, DiskNews};
pub(crate) use self::edit::{Gesture, gesture};
pub(crate) use self::view::{Pressed, activity};

const FADE: Duration = Duration::from_millis(450);
const FADE_FROM: f64 = 0.97;
const FOLDER_MARK: char = '/';

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Touched {
    files: BTreeMap<String, TouchedFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TouchedFile {
    created: bool,
    changes: Vec<Vec<Hunk>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Saving {
    Saved,
    Failed(WorkspaceError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) touched: Option<bool>,
    pub(crate) path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Editor {
    path: String,
    language: CodeLanguage,
    buffer: Buffer,
    versions: Versions,
    mine: Lines,
    folder: String,
    entries: Vec<Entry>,
    top: usize,
    visible: usize,
    opened: Instant,
    saving: Saving,
}

impl Touched {
    pub(crate) fn record(&mut self, change: &FileChange) {
        let file = self.files.entry(change.path.clone()).or_default();
        if change.created {
            *file = TouchedFile {
                created: true,
                changes: Vec::new(),
            };
            return;
        }
        file.changes.push(change.hunks.clone());
    }

    pub(crate) fn created(&self, path: &str) -> Option<bool> {
        self.files.get(path).map(|file| file.created)
    }
}

impl Editor {
    pub(crate) fn new(
        path: String,
        text: String,
        touched: &Touched,
        listed: Vec<String>,
        now: Instant,
    ) -> Self {
        let file = touched.files.get(&path).cloned().unwrap_or_default();
        let versions = Versions::new(text.clone(), &file.changes, file.created);
        let folder = folder_of(&path).to_owned();
        let entries = listed
            .into_iter()
            .map(|name| {
                let path = if folder.is_empty() {
                    name.clone()
                } else {
                    format!("{folder}{FOLDER_MARK}{name}")
                };
                let touched = touched.created(&path);
                Entry {
                    name,
                    touched,
                    path,
                }
            })
            .collect();
        Self {
            language: CodeLanguage::of(&path),
            buffer: Buffer::new(text, TextWidth::default()),
            mine: Lines::default(),
            versions,
            folder,
            entries,
            path,
            top: 0,
            visible: usize::MAX,
            opened: now,
            saving: Saving::Saved,
        }
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn text(&self) -> &str {
        self.buffer.text()
    }

    pub(crate) const fn saving(&self) -> &Saving {
        &self.saving
    }

    pub(crate) fn saved(&mut self, result: Result<(), WorkspaceError>) {
        self.saving = result.map_or_else(Saving::Failed, |()| Saving::Saved);
    }

    pub(crate) fn scroll(&mut self, rows: f64) {
        let last = self.buffer.rows().len().saturating_sub(1);
        let moved = dot_transcript::rows(self.top) + rows.round();
        self.top = dot_transcript::index(moved.max(0.0)).min(last);
    }

    pub(crate) fn is_opening(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.opened) < FADE
    }

    pub(crate) fn fade_in(&self, scene: &mut Scene, middle: [f32; 2], now: Instant) {
        let Some(share) = Glide::landing(FADE).eased(now.saturating_duration_since(self.opened))
        else {
            return;
        };
        scene.fade_around(
            middle,
            opacity(share),
            FADE_FROM + (1.0 - FADE_FROM) * share,
        );
        scene.animating = true;
    }

    fn fit(&mut self, width: TextWidth, visible: usize) {
        let visible = visible.max(1);
        let rows = self.buffer.rows().len();
        self.buffer.rewrap_to(width);
        if self.visible != visible || self.buffer.rows().len() != rows {
            self.visible = visible;
            self.keep_caret_in_sight();
        }
    }

    fn settle(&mut self, changed: bool) {
        if changed {
            self.mine = self.versions.mine(self.buffer.text());
        }
        self.keep_caret_in_sight();
    }

    fn keep_caret_in_sight(&mut self) {
        let (row, _) = self.buffer.caret_cell();
        if row < self.top {
            self.top = row;
        } else if row >= self.top.saturating_add(self.visible) {
            self.top = row + 1 - self.visible;
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/editor/mod.rs"]
pub(crate) mod tests;
