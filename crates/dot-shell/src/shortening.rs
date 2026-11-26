use std::path::Path;

use dot_api::{CUT_OUTPUT_NOTE, Compactor, Origin};
use dot_compact::{Burst, LineStream, ShapeCompactor};
use dot_process::Relay;

use crate::whole::{Kept, MOST_MEBIBYTES_KEPT, WholeOutputs};

const NEW_LINE: char = '\n';
const LINE_BREAK: u8 = b'\n';

/// [`Relay`] that shortens what a command of Claude prints.
///
/// An output that ended quickly is shortened whole, with every rule. An output still coming is
/// shortened line by line: without the colours of the terminal nor the lines redrawn in place,
/// and with the rules of the tools the command names; the contents of a file pass as they come.
/// Either way an output that is long is cut, the whole of it is kept in a file, and its last
/// line names the file; an output still coming names it as soon as something is cut. What is
/// not text is passed on as it is. Each of these is done only as far as the [`ShapeCompactor`]
/// does it: an output that is never cut is never kept in a file either.
pub struct Shortening {
    command: String,
    compactor: ShapeCompactor,
    stream: Option<LineStream>,
    cut_character: Vec<u8>,
    burst: Option<Burst>,
    whole_outputs: WholeOutputs,
    kept: Option<Kept>,
    lasts: bool,
    has_named_the_file: bool,
}

impl Shortening {
    /// Creates the relay of what `command`, as Claude wrote it, prints, shortened by
    /// `compactor`; an output that is cut is kept whole in `whole_outputs`.
    #[must_use]
    pub fn of(command: String, compactor: ShapeCompactor, whole_outputs: WholeOutputs) -> Self {
        Self {
            stream: compactor.stream(&command),
            burst: compactor.burst(&command),
            compactor,
            command,
            cut_character: Vec::new(),
            whole_outputs,
            kept: None,
            lasts: false,
            has_named_the_file: false,
        }
    }

    fn naming_the_whole(&self, shorter: String, output: &[u8]) -> String {
        let (told, line_break) = match shorter.strip_suffix(NEW_LINE) {
            Some(told) => (told, "\n"),
            None => (shorter.as_str(), ""),
        };
        let Some(before) = told.strip_suffix(CUT_OUTPUT_NOTE) else {
            return shorter;
        };
        let Ok(kept) = self.whole_outputs.keep(output) else {
            return shorter;
        };
        format!("{before}{}{line_break}", whole_in(&kept))
    }

    fn keep_as_it_comes(&mut self, piece: &[u8]) {
        if self.burst.is_none() {
            return;
        }
        if !self.lasts {
            self.lasts = true;
            self.kept = self.whole_outputs.start().ok();
        }
        let failed = self
            .kept
            .as_mut()
            .is_some_and(|kept| kept.write(piece).is_err());
        if failed {
            self.forget_what_is_kept();
        }
    }

    fn forget_what_is_kept(&mut self) {
        if let Some(kept) = self.kept.take() {
            kept.forget().unwrap_or_default();
        }
    }

    fn line_by_line(&mut self, piece: &[u8]) -> Vec<u8> {
        let Some(stream) = self.stream.as_mut() else {
            return piece.to_vec();
        };
        let mut bytes = std::mem::take(&mut self.cut_character);
        bytes.extend_from_slice(piece);

        let read = match std::str::from_utf8(&bytes) {
            Ok(text) => text.len(),
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => {
                let mut told = stream.rest().into_bytes();
                told.extend_from_slice(&bytes);
                self.stream = None;
                return told;
            }
        };
        let (text, cut) = bytes.split_at(read);
        self.cut_character = cut.to_vec();
        stream.push(&String::from_utf8_lossy(text)).into_bytes()
    }

    fn passed_at_a_pause(&mut self) -> Vec<u8> {
        let mut waiting = self
            .stream
            .as_mut()
            .map_or_else(Vec::new, |stream| stream.rest().into_bytes());
        waiting.append(&mut self.cut_character);
        self.cut(waiting)
    }

    fn cut(&mut self, told: Vec<u8>) -> Vec<u8> {
        match self.burst.as_mut() {
            Some(burst) => burst.pass(&told),
            None => told,
        }
    }

    fn pause(&mut self) -> Vec<u8> {
        self.burst.as_mut().map_or_else(Vec::new, Burst::pause)
    }

    fn has_cut(&self) -> bool {
        self.burst.as_ref().is_some_and(Burst::has_cut)
    }

    fn is_mid_line(&self) -> bool {
        self.burst.as_ref().is_some_and(Burst::is_mid_line)
    }

    fn where_the_whole_is_so_far(&mut self) -> Option<String> {
        if self.has_named_the_file || !self.has_cut() || self.is_mid_line() {
            return None;
        }
        let kept = self.kept.as_ref()?;
        self.has_named_the_file = true;
        if kept.is_whole() {
            return Some(whole_so_far_in(kept.path()));
        }
        Some(start_in(kept.path()))
    }

    fn where_the_whole_is(&mut self) -> Option<String> {
        if !self.has_cut() {
            self.forget_what_is_kept();
            return None;
        }
        match self.kept.take() {
            Some(kept) if kept.is_whole() => Some(whole_in(kept.path())),
            Some(kept) => Some(start_in(kept.path())),
            None => Some(CUT_OUTPUT_NOTE.to_owned()),
        }
    }
}

impl Relay for Shortening {
    fn whole(&mut self, output: &[u8]) -> Option<Vec<u8>> {
        let text = std::str::from_utf8(output).ok()?;
        let shorter = self
            .compactor
            .compact(Origin::Command(&self.command), text)?;
        Some(self.naming_the_whole(shorter, output).into_bytes())
    }

    fn piece(&mut self, piece: &[u8]) -> Vec<u8> {
        self.keep_as_it_comes(piece);
        let told = self.line_by_line(piece);
        self.cut(told)
    }

    fn rest(&mut self) -> Vec<u8> {
        let mut told = self.passed_at_a_pause();
        if let Some(note) = self.where_the_whole_is_so_far() {
            push_line(&mut told, &note);
        }
        told.append(&mut self.pause());
        told
    }

    fn last(&mut self) -> Vec<u8> {
        let mut told = self.passed_at_a_pause();
        told.append(&mut self.pause());
        if let Some(note) = self.where_the_whole_is() {
            if self.is_mid_line() {
                told.push(LINE_BREAK);
            }
            push_line(&mut told, &note);
        }
        told
    }
}

fn push_line(told: &mut Vec<u8>, line: &str) {
    told.extend_from_slice(line.as_bytes());
    told.push(LINE_BREAK);
}

fn whole_in(path: &Path) -> String {
    format!("[cut: the whole output is in {}]", path.display())
}

fn whole_so_far_in(path: &Path) -> String {
    format!("[cut: the whole output so far is in {}]", path.display())
}

fn start_in(path: &Path) -> String {
    format!(
        "[cut: the first {MOST_MEBIBYTES_KEPT} MiB of the output are in {}]",
        path.display()
    )
}

#[cfg(test)]
#[path = "../../../tests/dot-shell/shortening.rs"]
mod tests;
