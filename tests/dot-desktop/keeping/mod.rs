use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::{Hue, Journal, KeptSession, Record};

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
pub(crate) struct Shelved {
    pub(crate) shelf: Shelf,
    pub(crate) alive: Option<Vec<KeptId>>,
    pub(crate) arranged: Vec<Arrangement>,
    pub(crate) journals: BTreeMap<KeptId, Vec<Record<'static>>>,
    pub(crate) memories: BTreeMap<KeptId, Vec<u8>>,
    pub(crate) flushes: u32,
    pub(crate) refuses: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MemoryArchive {
    pub(crate) shelved: Arc<Mutex<Shelved>>,
}

#[derive(Debug)]
pub(crate) struct MemoryJournal {
    id: KeptId,
    shelved: Arc<Mutex<Shelved>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MemoryPrompts {
    pub(crate) loaded: Vec<String>,
    pub(crate) saved: Arc<Mutex<Vec<Vec<String>>>>,
}

impl MemoryArchive {
    pub(crate) fn with(shelf: Shelf) -> Self {
        let archive = Self::default();
        archive.shelved().shelf = shelf;
        archive
    }

    pub(crate) fn shelved(&self) -> std::sync::MutexGuard<'_, Shelved> {
        self.shelved.lock().unwrap()
    }
}

impl Archive for MemoryArchive {
    type Journal = MemoryJournal;

    fn restore(&self, alive: &[KeptId]) -> Shelf {
        let mut shelved = self.shelved();
        shelved.alive = Some(alive.to_vec());
        shelved.shelf.clone()
    }

    fn journal(&self, id: KeptId) -> MemoryJournal {
        MemoryJournal {
            id,
            shelved: Arc::clone(&self.shelved),
        }
    }

    fn arrange(&self, arrangement: &Arrangement) -> Result<(), ArchiveError> {
        let mut shelved = self.shelved();
        if shelved.refuses {
            return Err(ArchiveError("the disk is full".to_owned()));
        }
        shelved.arranged.push(arrangement.clone());
        Ok(())
    }
}

impl Journal for MemoryJournal {
    fn write(&mut self, record: &Record<'_>) -> Result<(), ArchiveError> {
        let mut shelved = self.shelved.lock().unwrap();
        if shelved.refuses {
            return Err(ArchiveError("the disk is full".to_owned()));
        }
        let owned = Record {
            at: record.at,
            happened: record.happened.clone().into_owned(),
        };
        shelved.journals.entry(self.id).or_default().push(owned);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), ArchiveError> {
        self.shelved.lock().unwrap().flushes += 1;
        Ok(())
    }

    fn keep_memory(
        &mut self,
        chat: &dyn Fn(&mut dyn Write) -> io::Result<()>,
    ) -> Result<(), ArchiveError> {
        let mut memory = Vec::new();
        chat(&mut memory)?;
        self.shelved
            .lock()
            .unwrap()
            .memories
            .insert(self.id, memory);
        Ok(())
    }
}

impl PromptHistory for MemoryPrompts {
    fn load(&self) -> Vec<String> {
        self.loaded.clone()
    }

    fn save(&self, prompts: &[String]) -> Result<(), ArchiveError> {
        self.saved.lock().unwrap().push(prompts.to_vec());
        Ok(())
    }
}

pub(crate) fn wait_for<T>(received: &Receiver<T>) -> T {
    received.recv_timeout(PATIENCE).unwrap()
}

fn kept(id: u64) -> KeptSession {
    KeptSession {
        id: KeptId::new(id),
        name: None,
        hue: Hue::Cyan,
        folder: None,
        model: None,
        effort: None,
    }
}

type TestKeeping = Keeping<MemoryArchive, MemoryPrompts, Sender<KeptNews>>;

fn keeping(archive: MemoryArchive, prompts: MemoryPrompts) -> (TestKeeping, Receiver<KeptNews>) {
    let (sender, received) = mpsc::channel();
    (Keeping::new(Arc::new(archive), prompts, sender), received)
}

fn arrangement(ids: &[u64]) -> Arrangement {
    Arrangement {
        sessions: ids.iter().map(|id| kept(*id)).collect(),
        shown: ids.first().map(|id| KeptId::new(*id)),
    }
}

#[test]
fn restores_the_shelf_and_loads_the_prompts_once_started() {
    let shelf = Shelf {
        sessions: Vec::new(),
        shown: Some(KeptId::new(4)),
    };
    let archive = MemoryArchive::with(shelf.clone());
    let prompts = MemoryPrompts {
        loaded: vec!["older".to_owned()],
        ..MemoryPrompts::default()
    };
    let (mut keeping, received) = keeping(archive.clone(), prompts);

    keeping.start(vec![KeptId::new(9)]);
    let mut news = [wait_for(&received), wait_for(&received)];
    news.sort_by_key(|news| matches!(news, KeptNews::Prompts(_)));

    assert_eq!(
        news,
        [
            KeptNews::Restored(shelf),
            KeptNews::Prompts(vec!["older".to_owned()])
        ]
    );
    assert_eq!(archive.shelved().alive, Some(vec![KeptId::new(9)]));
}

#[test]
fn arranges_nothing_before_the_restore_has_landed() {
    let archive = MemoryArchive::default();
    let (mut keeping, received) = keeping(archive.clone(), MemoryPrompts::default());

    keeping.arrange(arrangement(&[1]));
    assert!(!keeping.has_restored());
    keeping.restored();
    keeping.arrange(arrangement(&[2]));
    assert_eq!(wait_for(&received), KeptNews::Kept);

    assert_eq!(archive.shelved().arranged, [arrangement(&[2])]);
    assert_eq!(keeping.last_arrangement(), Some(&arrangement(&[2])));
    assert!(keeping.has_restored());
}

#[test]
fn saves_no_prompt_before_the_kept_ones_are_loaded_nor_twice_the_same() {
    let prompts = MemoryPrompts::default();
    let (mut keeping, received) = keeping(MemoryArchive::default(), prompts.clone());
    let typed = vec!["fix it".to_owned()];

    keeping.keep_prompts(&typed, 1);
    keeping.loaded();
    keeping.keep_prompts(&typed, 1);
    assert_eq!(wait_for(&received), KeptNews::Kept);
    keeping.keep_prompts(&typed, 1);
    drop(keeping);

    assert_eq!(*prompts.saved.lock().unwrap(), [typed]);
}

#[test]
fn an_older_write_never_lands_after_a_newer_one() {
    let done = Mutex::new(0);
    let mut written = Vec::new();

    unless_outdated(&done, 2, || {
        written.push(2);
        Ok(())
    });
    unless_outdated(&done, 1, || {
        written.push(1);
        Ok(())
    });
    unless_outdated(&done, 3, || {
        Err(ArchiveError("the disk is full".to_owned()))
    });
    unless_outdated(&done, 3, || {
        written.push(3);
        Ok(())
    });

    assert_eq!(written, [2, 3]);
}

#[test]
fn numbers_each_write_after_the_one_before() {
    let mut written = Written::default();

    assert_eq!([written.next(), written.next(), written.next()], [1, 2, 3]);
}
