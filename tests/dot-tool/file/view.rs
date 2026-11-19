use std::fs::{self, OpenOptions};
use std::time::Instant;

use dot_api::ToolEnd;

use super::*;
use crate::toolbox::tests::{TempFolder, without_waiting};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n rest of a tiny image";

fn text(output: &ToolOutput) -> &str {
    match &output.content {
        ToolContent::Text(text) => text,
        ToolContent::Image(_) => panic!("expected text"),
    }
}

fn numbered(count: u64) -> String {
    (1..=count).fold(String::new(), |mut text, line| {
        text.push_str("line ");
        text.push_str(&line.to_string());
        text.push('\n');
        text
    })
}

fn range(first: u64, last: Option<u64>) -> LineRange {
    LineRange { first, last }
}

#[test]
fn numbers_each_line_in_six_columns_then_a_tab() {
    let folder = TempFolder::new("view-numbers");
    let file = folder.write("a.txt", "fn main() {\r\n}\n");

    let output = view(&file, None);

    assert_eq!(text(&output), "     1\tfn main() {\n     2\t}\n");
    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(2)));
    assert!(!output.is_error);
}

#[test]
fn shows_the_lines_of_a_range() {
    let folder = TempFolder::new("view-range");
    let file = folder.write("a.txt", &numbered(10));

    let middle = view(&file, Some(range(3, Some(4))));
    let to_the_end = view(&file, Some(range(9, None)));
    let past_the_end = view(&file, Some(range(9, Some(50))));

    assert_eq!(text(&middle), "     3\tline 3\n     4\tline 4\n");
    assert_eq!(middle.end, ToolEnd::Done(Detail::Lines(2)));
    assert_eq!(text(&to_the_end), "     9\tline 9\n    10\tline 10\n");
    assert_eq!(text(&past_the_end), text(&to_the_end));
}

#[test]
fn refuses_ranges_that_start_at_zero_end_before_they_start_or_start_past_the_end() {
    let folder = TempFolder::new("view-bad-range");
    let file = folder.write("a.txt", &numbered(3));

    let zero = view(&file, Some(range(0, None)));
    let backwards = view(&file, Some(range(3, Some(2))));
    let past = view(&file, Some(range(4, None)));
    let last_line = view(&file, Some(range(3, Some(3))));

    assert_eq!(zero.end, ToolEnd::Failed);
    assert!(text(&zero).contains("counted from 1"));
    assert_eq!(backwards.end, ToolEnd::Failed);
    assert_eq!(past.end, ToolEnd::Failed);
    assert!(text(&past).contains("has 3 lines: line 4 is past its end"));
    assert_eq!(text(&last_line), "     3\tline 3\n");
}

#[test]
fn shows_two_thousand_lines_at_most_and_says_where_to_go_on() {
    let folder = TempFolder::new("view-cap");
    let file = folder.write("a.txt", &numbered(2_001));
    let exact = folder.write("b.txt", &numbered(2_000));

    let capped = view(&file, None);
    let ranged = view(&file, Some(range(1, Some(2_001))));
    let whole = view(&exact, None);

    assert_eq!(capped.end, ToolEnd::Done(Detail::Lines(2_000)));
    assert!(text(&capped).ends_with(
        "  2000\tline 2000\n[Lines after 2000 are not shown: view the file again with a range \
         that starts at line 2001.]"
    ));
    assert_eq!(ranged.end, ToolEnd::Done(Detail::Lines(2_000)));
    assert!(text(&ranged).contains("starts at line 2001"));
    assert_eq!(whole.end, ToolEnd::Done(Detail::Lines(2_000)));
    assert!(!text(&whole).contains("not shown"));
}

#[test]
fn a_closed_range_that_ends_inside_the_file_says_nothing_more() {
    let folder = TempFolder::new("view-closed");
    let file = folder.write("a.txt", &numbered(5));

    let output = view(&file, Some(range(1, Some(2))));

    assert!(!text(&output).contains("not shown"));
}

#[test]
fn cuts_a_line_past_two_thousand_characters() {
    let folder = TempFolder::new("view-long-line");
    let long = "é".repeat(2_001);
    let exact = "é".repeat(2_000);
    let file = folder.write("a.txt", &format!("{long}\n{exact}\n"));

    let output = view(&file, None);
    let lines: Vec<&str> = text(&output).lines().collect();

    assert_eq!(lines[0], format!("     1\t{exact} […]"));
    assert_eq!(lines[1], format!("     2\t{exact}"));
}

#[test]
fn says_when_a_file_is_empty() {
    let folder = TempFolder::new("view-empty");
    let file = folder.write("empty.txt", "");

    let output = view(&file, None);

    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(0)));
    assert!(text(&output).ends_with("empty.txt is empty."));
    assert_eq!(view(&file, Some(range(2, None))).end, ToolEnd::Failed);
}

#[test]
fn hands_an_image_over_as_an_image() {
    let folder = TempFolder::new("view-image");
    let file = folder.join("shot.png");
    fs::write(&file, PNG).unwrap();

    let output = view(&file, None);

    assert_eq!(
        output.content,
        ToolContent::Image(Image::from_bytes(PNG.to_vec()).unwrap())
    );
    assert_eq!(output.end, ToolEnd::Done(Detail::Nothing));
}

#[test]
fn refuses_an_image_larger_than_claude_reads() {
    let folder = TempFolder::new("view-big-image");
    let file = folder.join("big.png");
    fs::write(&file, PNG).unwrap();
    let limit = u64::try_from(Image::LARGEST).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_len(limit + 1)
        .unwrap();
    let at_the_limit = folder.join("limit.png");
    fs::copy(&file, &at_the_limit).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&at_the_limit)
        .unwrap()
        .set_len(limit)
        .unwrap();

    let too_big = view(&file, None);
    let just_fits = view(&at_the_limit, None);

    assert_eq!(too_big.end, ToolEnd::Failed);
    assert!(text(&too_big).contains("larger than 5 MiB"));
    assert_eq!(just_fits.end, ToolEnd::Done(Detail::Nothing));
}

#[test]
fn refuses_a_binary_file() {
    let folder = TempFolder::new("view-binary");
    let file = folder.join("data.bin");
    fs::write(&file, b"ELF\0\x01\x02").unwrap();

    let output = view(&file, None);

    assert_eq!(output.end, ToolEnd::Failed);
    assert!(text(&output).contains("binary"));
}

#[test]
fn a_nul_past_the_sniffed_start_still_reads_as_text() {
    let folder = TempFolder::new("view-late-nul");
    let file = folder.join("late.txt");
    let mut bytes = vec![b'a'; usize::try_from(SNIFFED).unwrap()];
    bytes.extend_from_slice(b"\0\n");
    fs::write(&file, bytes).unwrap();

    let output = view(&file, None);

    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(1)));
}

#[test]
fn lists_a_folder_and_fails_on_what_is_missing() {
    let folder = TempFolder::new("view-folder");
    folder.write("a.txt", "");

    let listed = view(folder.path(), None);
    let missing = view(&folder.join("missing.txt"), None);

    assert_eq!(listed.end, ToolEnd::Done(Detail::Files(1)));
    assert_eq!(missing.end, ToolEnd::Failed);
    assert!(text(&missing).contains("Cannot read"));
}

#[test]
fn reads_only_the_start_of_a_huge_file() {
    let folder = TempFolder::new("view-huge");
    let file = folder.write("huge.txt", &numbered(3_000));
    OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .set_len(4 * 1_024 * 1_024 * 1_024)
        .unwrap();

    let started = Instant::now();
    let ranged = view(&file, Some(range(1, Some(2))));
    let capped = view(&file, None);

    assert_eq!(text(&ranged), "     1\tline 1\n     2\tline 2\n");
    assert_eq!(capped.end, ToolEnd::Done(Detail::Lines(2_000)));
    assert!(text(&capped).contains("starts at line 2001"));
    assert!(started.elapsed().as_millis() < 500);
}

#[test]
fn refuses_a_named_pipe_at_once() {
    let folder = TempFolder::new("view-pipe");
    let pipe = folder.pipe("fifo");

    let output = without_waiting(move || view(&pipe, None));

    assert!(output.is_error);
    assert!(text(&output).contains("is not a regular file"));
}

#[test]
fn shows_at_most_256_kibibytes_then_says_where_to_go_on() {
    let folder = TempFolder::new("view-bytes");
    let line = format!("{}\n", "x".repeat(199));
    let file = folder.write("wide.txt", &line.repeat(2_000));

    let output = view(&file, None);

    let shown = text(&output);
    assert!(shown.starts_with("     1\txxx"));
    assert!(shown.ends_with("view the file again with a range that starts at line 1268.]"));
    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(1_267)));
}

#[test]
fn shows_a_file_just_under_256_kibibytes_whole() {
    let folder = TempFolder::new("view-under");
    let line = format!("{}\n", "x".repeat(199));
    let file = folder.write("wide.txt", &line.repeat(1_266));

    let output = view(&file, None);

    assert!(!text(&output).contains("are not shown"));
    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(1_266)));
}
