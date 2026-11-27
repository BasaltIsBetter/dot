use std::sync::{Arc, Mutex, PoisonError};

use dot_api::{Archive, ArchiveError, Arrangement, KeptId, PromptHistory, Shelf};

use crate::worker::{Mailbox, Workers};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum KeptNews {
    Restored(Shelf),
    Prompts(Vec<String>),
    Kept,
}

pub(crate) struct Keeping<A, R, M> {
    archive: Arc<A>,
    prompts: Arc<R>,
    workers: Workers<M>,
    arranged: Written,
    saved: Written,
    restored: bool,
    loaded: bool,
    last: Option<Arrangement>,
    prompts_kept: u64,
}

#[derive(Default)]
struct Written {
    done: Arc<Mutex<u64>>,
    asked: u64,
}

impl<A, R, M> Keeping<A, R, M>
where
    A: Archive + 'static,
    R: PromptHistory + 'static,
    M: Mailbox<KeptNews>,
{
    pub(crate) fn new(archive: Arc<A>, prompts: R, mailbox: M) -> Self {
        Self {
            archive,
            prompts: Arc::new(prompts),
            workers: Workers::new(mailbox),
            arranged: Written::default(),
            saved: Written::default(),
            restored: false,
            loaded: false,
            last: None,
            prompts_kept: 0,
        }
    }

    pub(crate) fn start(&mut self, alive: Vec<KeptId>) {
        let archive = Arc::clone(&self.archive);
        self.workers.spawn(
            move |_| KeptNews::Restored(archive.restore(&alive)),
            || Some(KeptNews::Restored(Shelf::default())),
        );
        let prompts = Arc::clone(&self.prompts);
        self.workers.spawn(
            move |_| KeptNews::Prompts(prompts.load()),
            || Some(KeptNews::Prompts(Vec::new())),
        );
    }

    pub(crate) const fn has_restored(&self) -> bool {
        self.restored
    }

    pub(crate) const fn restored(&mut self) {
        self.restored = true;
    }

    pub(crate) const fn loaded(&mut self) {
        self.loaded = true;
    }

    pub(crate) fn last_arrangement(&self) -> Option<&Arrangement> {
        self.last.as_ref()
    }

    pub(crate) fn arrange(&mut self, arrangement: Arrangement) {
        if !self.restored {
            return;
        }
        self.last = Some(arrangement.clone());
        let generation = self.arranged.next();
        let archive = Arc::clone(&self.archive);
        let done = Arc::clone(&self.arranged.done);
        self.workers.spawn(
            move |_| {
                unless_outdated(&done, generation, || archive.arrange(&arrangement));
                KeptNews::Kept
            },
            || Some(KeptNews::Kept),
        );
    }

    pub(crate) fn keep_prompts(&mut self, prompts: &[String], changes: u64) {
        if !self.loaded || changes == self.prompts_kept {
            return;
        }
        self.prompts_kept = changes;
        let generation = self.saved.next();
        let history = Arc::clone(&self.prompts);
        let done = Arc::clone(&self.saved.done);
        let prompts = prompts.to_vec();
        self.workers.spawn(
            move |_| {
                unless_outdated(&done, generation, || history.save(&prompts));
                KeptNews::Kept
            },
            || Some(KeptNews::Kept),
        );
    }
}

impl Written {
    const fn next(&mut self) -> u64 {
        self.asked += 1;
        self.asked
    }
}

fn unless_outdated(
    done: &Mutex<u64>,
    generation: u64,
    write: impl FnOnce() -> Result<(), ArchiveError>,
) {
    let mut last = done.lock().unwrap_or_else(PoisonError::into_inner);
    if *last > generation {
        return;
    }
    *last = generation;
    if let Err(error) = write() {
        tracing::warn!(%error, "the sessions or the prompts were not kept");
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/keeping/mod.rs"]
pub(crate) mod tests;
