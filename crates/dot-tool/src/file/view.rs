use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

use dot_api::{Detail, Image, LineRange, ToolContent, ToolEnd, ToolOutput};

use crate::file::store::{not_a_file, open_without_waiting};
use crate::file::{SNIFFED, folder, is_binary, read_line};
use crate::text::append;

const MOST_LINES: u64 = 2_000;
const MOST_SHOWN_BYTES: usize = 256 * 1_024;
const LONGEST_LINE: usize = 2_000;
const BYTES_PER_CHARACTER: usize = 4;
const NUMBER_WIDTH: usize = 6;
const CUT_MARK: &str = " […]";

struct Shown {
    text: String,
    first: u64,
    last: Option<u64>,
    total_reached: u64,
    more: bool,
}

pub(crate) fn view(path: &Path, lines: Option<LineRange>) -> ToolOutput {
    let (file, metadata) = match open_without_waiting(path) {
        Ok(opened) => opened,
        Err(error) => {
            return ToolOutput::failed(format!("Cannot read {}: {error}.", path.display()));
        }
    };
    if metadata.is_dir() {
        return folder::list(path);
    }
    if !metadata.is_file() {
        return ToolOutput::failed(not_a_file(path));
    }
    match read(path, file, metadata.len(), lines) {
        Ok(output) | Err(output) => output,
    }
}

fn read(
    path: &Path,
    mut file: File,
    size: u64,
    lines: Option<LineRange>,
) -> Result<ToolOutput, ToolOutput> {
    let unreadable =
        |error| ToolOutput::failed(format!("Cannot read {}: {error}.", path.display()));
    let mut head = Vec::new();
    file.by_ref()
        .take(SNIFFED)
        .read_to_end(&mut head)
        .map_err(unreadable)?;

    if Image::from_bytes(head.clone()).is_ok() {
        return image(path, size, head, file);
    }
    if is_binary(&head) {
        return Err(ToolOutput::failed(format!(
            "{} is a binary file, not text: Claude cannot read it this way.",
            path.display()
        )));
    }
    let range = lines.unwrap_or(LineRange {
        first: 1,
        last: None,
    });
    check(range)?;
    let shown =
        number_lines(BufReader::new(head.as_slice().chain(file)), range).map_err(unreadable)?;
    finish(path, shown)
}

fn image(
    path: &Path,
    size: u64,
    mut bytes: Vec<u8>,
    mut rest: File,
) -> Result<ToolOutput, ToolOutput> {
    if size > u64::try_from(Image::LARGEST).unwrap_or(u64::MAX) {
        return Err(ToolOutput::failed(format!(
            "{} is an image larger than 5 MiB, the limit of Claude for one image.",
            path.display()
        )));
    }
    rest.read_to_end(&mut bytes)
        .map_err(|error| ToolOutput::failed(format!("Cannot read {}: {error}.", path.display())))?;
    let image = Image::from_bytes(bytes)
        .map_err(|error| ToolOutput::failed(format!("{}: {error}.", path.display())))?;
    Ok(ToolOutput {
        content: ToolContent::Image(image),
        is_error: false,
        end: ToolEnd::Done(Detail::Nothing),
    })
}

fn check(range: LineRange) -> Result<(), ToolOutput> {
    if range.first == 0 {
        return Err(ToolOutput::failed(
            "Lines are counted from 1: start the range at 1 or after.",
        ));
    }
    if range.last.is_some_and(|last| last < range.first) {
        return Err(ToolOutput::failed("The range ends before it starts."));
    }
    Ok(())
}

fn number_lines(mut reader: impl BufRead, range: LineRange) -> io::Result<Shown> {
    let ceiling = range.first.saturating_add(MOST_LINES - 1);
    let stop = range.last.map_or(ceiling, |last| last.min(ceiling));
    let mut shown = Shown {
        text: String::new(),
        first: range.first,
        last: None,
        total_reached: 0,
        more: false,
    };
    let mut line = Vec::new();
    let mut number = 0_u64;

    while let Some(cut) = read_line(&mut reader, &mut line, LONGEST_LINE * BYTES_PER_CHARACTER)? {
        number += 1;
        shown.total_reached = number;
        if number < range.first {
            continue;
        }
        push_line(&mut shown.text, number, &line, cut);
        shown.last = Some(number);
        if number == stop || shown.text.len() >= MOST_SHOWN_BYTES {
            let rest = !reader.fill_buf()?.is_empty();
            shown.more = rest && range.last.is_none_or(|last| number < last);
            break;
        }
    }
    Ok(shown)
}

fn push_line(text: &mut String, number: u64, line: &[u8], cut: bool) {
    let line = String::from_utf8_lossy(line);
    let kept: String = line.chars().take(LONGEST_LINE).collect();
    let cut = cut || kept.len() < line.len();
    let mark = if cut { CUT_MARK } else { "" };
    append(
        text,
        format_args!("{number:>NUMBER_WIDTH$}\t{kept}{mark}\n"),
    );
}

fn finish(path: &Path, mut shown: Shown) -> Result<ToolOutput, ToolOutput> {
    let Some(last) = shown.last else {
        if shown.total_reached == 0 && shown.first == 1 {
            return Ok(ToolOutput::done(
                format!("{} is empty.", path.display()),
                Detail::Lines(0),
            ));
        }
        return Err(ToolOutput::failed(format!(
            "{} has {} lines: line {} is past its end.",
            path.display(),
            shown.total_reached,
            shown.first
        )));
    };
    if shown.more {
        let next = last + 1;
        append(
            &mut shown.text,
            format_args!(
                "[Lines after {last} are not shown: view the file again with a range that starts at line {next}.]"
            ),
        );
    }
    Ok(ToolOutput::done(
        shown.text,
        Detail::Lines(last - shown.first + 1),
    ))
}

#[cfg(test)]
#[path = "../../../../tests/dot-tool/file/view.rs"]
mod tests;
