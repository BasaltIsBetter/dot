use std::io::Read;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use dot_api::{Image, ImageError, Workspace, WorkspaceError};
use dot_process::open_without_waiting;

use crate::worker::{Mailbox, Workers, join};

const FOLDER_MARK: char = '/';
const WRITER_FELL: &str = "the writer stopped";
const READER_FELL: &str = "the file could not be read";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DiskNews {
    Opened {
        path: String,
        text: Result<String, WorkspaceError>,
        entries: Vec<String>,
    },
    Saved {
        path: String,
        result: Result<(), WorkspaceError>,
    },
    Dropped {
        name: String,
        image: Result<Vec<u8>, ImageError>,
    },
}

pub(crate) struct Disk<W, M> {
    workspace: Arc<W>,
    mailbox: M,
    workers: Workers<M>,
    saver: Option<Saver>,
}

struct Saver {
    waiting: Arc<Waiting>,
    handle: JoinHandle<()>,
}

#[derive(Default)]
struct Waiting {
    texts: Mutex<Pending>,
    arrived: Condvar,
}

#[derive(Default)]
struct Pending {
    latest: Vec<(String, String)>,
    finished: bool,
}

impl<W: Workspace + 'static, M: Mailbox<DiskNews>> Disk<W, M> {
    pub(crate) fn new(workspace: Arc<W>, mailbox: M) -> Self {
        Self {
            workspace,
            workers: Workers::new(mailbox.clone()),
            mailbox,
            saver: None,
        }
    }

    pub(crate) fn open(&mut self, path: String) {
        let workspace = Arc::clone(&self.workspace);
        let crashed_path = path.clone();
        self.workers.spawn(
            move |_| {
                let text = workspace.read(&path);
                let entries = workspace.list(folder_of(&path)).unwrap_or_default();
                DiskNews::Opened {
                    path,
                    text,
                    entries,
                }
            },
            move || {
                Some(DiskNews::Opened {
                    path: crashed_path,
                    text: Err(WorkspaceError::Failed(READER_FELL.to_owned())),
                    entries: Vec::new(),
                })
            },
        );
    }

    pub(crate) fn read_image(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let crashed_name = name.clone();
        self.workers.spawn(
            move |_| DiskNews::Dropped {
                name,
                image: read_image(&path),
            },
            move || {
                Some(DiskNews::Dropped {
                    name: crashed_name,
                    image: Err(ImageError::Unsupported),
                })
            },
        );
    }

    pub(crate) fn save(&mut self, path: &str, text: &str) {
        let saver = self.saver.get_or_insert_with(|| {
            let waiting = Arc::new(Waiting::default());
            let taken = Arc::clone(&waiting);
            let workspace = Arc::clone(&self.workspace);
            let mailbox = self.mailbox.clone();
            let handle = thread::spawn(move || write_each(&*workspace, &taken, &mailbox));
            Saver { waiting, handle }
        });
        keep_latest(&mut saver.waiting.pending().latest, path, text);
        saver.waiting.arrived.notify_one();
    }
}

impl<W, M> Disk<W, M> {
    pub(crate) fn finish(&mut self) {
        if let Some(Saver { waiting, handle }) = self.saver.take() {
            waiting.pending().finished = true;
            waiting.arrived.notify_one();
            join(handle);
        }
        self.workers.stop_all();
    }
}

impl<W, M> Drop for Disk<W, M> {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Waiting {
    fn pending(&self) -> MutexGuard<'_, Pending> {
        self.texts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn next(&self) -> Option<Vec<(String, String)>> {
        let mut pending = self
            .arrived
            .wait_while(self.pending(), |pending| {
                pending.latest.is_empty() && !pending.finished
            })
            .unwrap_or_else(PoisonError::into_inner);
        let latest = std::mem::take(&mut pending.latest);
        (!latest.is_empty()).then_some(latest)
    }
}

fn read_image(path: &Path) -> Result<Vec<u8>, ImageError> {
    let Ok((file, metadata)) = open_without_waiting(path) else {
        return Err(ImageError::Unsupported);
    };
    if !metadata.is_file() {
        return Err(ImageError::Unsupported);
    }
    let mut bytes = Vec::new();
    let longest = u64::try_from(Image::LARGEST).unwrap_or(u64::MAX);
    let read = file.take(longest + 1).read_to_end(&mut bytes);
    match read {
        Ok(_) => Ok(bytes),
        Err(_) => Err(ImageError::Unsupported),
    }
}

fn keep_latest(latest: &mut Vec<(String, String)>, path: &str, text: &str) {
    match latest.iter_mut().find(|(waiting, _)| waiting == path) {
        Some((_, waiting)) => text.clone_into(waiting),
        None => latest.push((path.to_owned(), text.to_owned())),
    }
}

pub(crate) fn folder_of(path: &str) -> &str {
    path.rsplit_once(FOLDER_MARK)
        .map_or("", |(folder, _)| folder)
}

fn write_each<W: Workspace>(workspace: &W, waiting: &Waiting, mailbox: &impl Mailbox<DiskNews>) {
    while let Some(latest) = waiting.next() {
        for text in latest {
            write(workspace, text, mailbox);
        }
    }
}

fn write<W: Workspace>(
    workspace: &W,
    (path, text): (String, String),
    mailbox: &impl Mailbox<DiskNews>,
) {
    let result = panic::catch_unwind(AssertUnwindSafe(|| workspace.write(&path, &text)))
        .unwrap_or_else(|_| Err(WorkspaceError::Failed(WRITER_FELL.to_owned())));
    if let Err(error) = &result {
        tracing::warn!(%error, "a file touched up in Dot was not saved");
    }
    mailbox.deliver(DiskNews::Saved { path, result });
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/editor/disk.rs"]
mod tests;
