use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use dot_api::{Workspace, WorkspaceError};

use crate::worker::{Mailbox, Workers, join};

const FOLDER_MARK: char = '/';
const WRITER_FELL: &str = "the writer stopped";

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
}

pub(crate) struct Disk<W, M> {
    workspace: Arc<W>,
    mailbox: M,
    workers: Workers<M>,
    saver: Option<Saver>,
}

struct Saver {
    sender: Sender<(String, String)>,
    handle: JoinHandle<()>,
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
            || None,
        );
    }

    pub(crate) fn save(&mut self, path: &str, text: &str) {
        let saver = self.saver.get_or_insert_with(|| {
            let (sender, receiver) = mpsc::channel();
            let workspace = Arc::clone(&self.workspace);
            let mailbox = self.mailbox.clone();
            let handle = thread::spawn(move || write_each(&*workspace, &receiver, &mailbox));
            Saver { sender, handle }
        });
        if saver
            .sender
            .send((path.to_owned(), text.to_owned()))
            .is_err()
        {
            tracing::warn!("{WRITER_FELL}");
        }
    }
}

impl<W, M> Drop for Disk<W, M> {
    fn drop(&mut self) {
        if let Some(Saver { sender, handle }) = self.saver.take() {
            drop(sender);
            join(handle);
        }
    }
}

pub(crate) fn folder_of(path: &str) -> &str {
    path.rsplit_once(FOLDER_MARK)
        .map_or("", |(folder, _)| folder)
}

fn write_each<W: Workspace>(
    workspace: &W,
    receiver: &Receiver<(String, String)>,
    mailbox: &impl Mailbox<DiskNews>,
) {
    while let Ok(mut latest) = receiver.recv() {
        while let Ok(next) = receiver.try_recv() {
            if next.0 != latest.0 {
                write(workspace, latest, mailbox);
            }
            latest = next;
        }
        write(workspace, latest, mailbox);
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
