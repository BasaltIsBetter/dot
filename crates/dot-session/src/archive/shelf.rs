use std::collections::VecDeque;
use std::io::{BufReader, Write};
use std::path::Path;
use std::time::Duration;

use dot_api::{Carried, Happened, Record};
use dot_process::{Line, read_line};

use crate::archive::journal::{HEADER, JOURNAL, MEMORY, is_header, line_of, record_of};
use crate::private::{open_ordinary, read_at_most, write_whole};

pub(crate) const LARGEST_JOURNAL: usize = 32 * 1_024 * 1_024;
pub(crate) const LONGEST_RECORD: usize = 8 * 1_024 * 1_024;
pub(crate) const LARGEST_MEMORY: u64 = 40 * 1_024 * 1_024;
const NEW_LINE: u8 = b'\n';

#[derive(Debug)]
struct Window {
    largest: usize,
    records: VecDeque<(Record<'static>, usize)>,
    bytes: usize,
    carried: Option<Carried>,
    latest: Duration,
}

pub(crate) fn records_of(folder: &Path) -> Vec<Record<'static>> {
    records_within(folder, LARGEST_JOURNAL)
}

fn records_within(folder: &Path, largest: usize) -> Vec<Record<'static>> {
    let Some(file) = open_ordinary(&folder.join(JOURNAL)) else {
        return Vec::new();
    };
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let opened = read_line(&mut reader, &mut line, LONGEST_RECORD);
    if !matches!(opened, Ok(Some(Line::Whole))) || !is_header(&line) {
        return Vec::new();
    }

    let mut window = Window::new(largest);
    loop {
        match read_line(&mut reader, &mut line, LONGEST_RECORD) {
            Ok(Some(Line::Whole)) => {
                if let Some(record) = record_of(&line) {
                    window.push(record, line.len());
                }
            }
            Ok(Some(Line::Cut)) => {}
            Ok(None) | Err(_) => break,
        }
    }
    window.into_records(folder)
}

pub(crate) fn memory_of(folder: &Path) -> Option<Vec<u8>> {
    memory_within(folder, LARGEST_MEMORY)
}

fn memory_within(folder: &Path, largest: u64) -> Option<Vec<u8>> {
    let memory = read_at_most(&folder.join(MEMORY), largest + 1)?;
    let fits = u64::try_from(memory.len()).is_ok_and(|length| length <= largest);
    fits.then_some(memory)
}

impl Window {
    const fn new(largest: usize) -> Self {
        Self {
            largest,
            records: VecDeque::new(),
            bytes: 0,
            carried: None,
            latest: Duration::ZERO,
        }
    }

    fn push(&mut self, mut record: Record<'static>, size: usize) {
        record.at = record.at.max(self.latest);
        self.latest = record.at;
        self.records.push_back((record, size));
        self.bytes += size;
        while self.bytes > self.largest && self.carry_the_first_exchange() {}
    }

    fn carry_the_first_exchange(&mut self) -> bool {
        let Some(next) = self
            .records
            .iter()
            .skip(1)
            .position(|(record, _)| record.happened.opens_an_exchange())
        else {
            return false;
        };
        let carried = self.carried.get_or_insert_with(Carried::default);
        for (record, size) in self.records.drain(..=next) {
            carried.add(&record.happened);
            self.bytes -= size;
        }
        true
    }

    fn into_records(self, folder: &Path) -> Vec<Record<'static>> {
        let mut records: Vec<Record<'static>> =
            self.records.into_iter().map(|(record, _)| record).collect();
        let Some(carried) = self.carried else {
            return records;
        };
        let at = records.first().map_or(self.latest, |first| first.at);
        records.insert(
            0,
            Record {
                at,
                happened: Happened::Carried(carried),
            },
        );
        if let Err(error) = write_whole(folder, JOURNAL, |file| write_journal(file, &records)) {
            tracing::warn!(%error, "a journal grown too large is read cut short, but not written so");
        }
        records
    }
}

fn write_journal(file: &mut dyn Write, records: &[Record<'_>]) -> std::io::Result<()> {
    file.write_all(HEADER)?;
    file.write_all(&[NEW_LINE])?;
    let mut line = Vec::new();
    for record in records {
        line_of(record, &mut line).map_err(|error| std::io::Error::other(error.0))?;
        file.write_all(&line)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/dot-session/archive/shelf.rs"]
mod tests;
