use proptest::prelude::*;

use super::*;
use crate::cut::{CONTENT, OTHER};

fn numbered(lines: std::ops::RangeInclusive<usize>) -> Vec<u8> {
    lines
        .map(|number| format!("line {number:04} of the output\n"))
        .collect::<Vec<String>>()
        .concat()
        .into_bytes()
}

#[test]
fn passes_the_start_of_a_burst_and_keeps_its_end_for_the_pause() {
    let mut burst = Burst::within(OTHER);

    let at_once = burst.pass(&numbered(1..=300));
    let at_the_pause = burst.pause();

    assert_eq!(at_once, numbered(1..=63));
    assert_eq!(
        at_the_pause,
        [b"[+175 lines cut]\n".to_vec(), numbered(239..=300)].concat()
    );
    assert!(burst.has_cut());
}

#[test]
fn keeps_more_of_the_contents_of_a_file() {
    let mut burst = Burst::within(CONTENT);

    let at_once = burst.pass(&numbered(1..=600));
    let at_the_pause = burst.pause();

    assert_eq!(at_once, numbered(1..=167));
    assert_eq!(
        at_the_pause,
        [b"[+350 lines cut]\n".to_vec(), numbered(518..=600)].concat()
    );
}

#[test]
fn tells_whole_a_burst_whose_end_is_no_longer_than_it_may_keep() {
    let mut burst = Burst::within(OTHER);
    let end = [numbered(64..=292), b"abc\n".to_vec()].concat();

    let at_once = burst.pass(&[numbered(1..=63), end.clone()].concat());
    let at_the_pause = burst.pause();

    assert_eq!(end.len(), 5_500);
    assert_eq!(at_once, numbered(1..=63));
    assert_eq!(at_the_pause, end);
    assert!(!burst.has_cut());
}

#[test]
fn cuts_a_burst_whose_end_is_one_byte_longer() {
    let mut burst = Burst::within(OTHER);
    let end = [numbered(64..=292), b"abcd\n".to_vec()].concat();

    let at_once = burst.pass(&[numbered(1..=63), end.clone()].concat());
    let at_the_pause = burst.pause();

    assert_eq!(end.len(), 5_501);
    assert_eq!(at_once, numbered(1..=63));
    assert_eq!(
        at_the_pause,
        [
            b"[+167 lines cut]\n".to_vec(),
            numbered(231..=292),
            b"abcd\n".to_vec()
        ]
        .concat()
    );
    assert!(burst.has_cut());
}

#[test]
fn starts_afresh_after_a_pause() {
    let mut burst = Burst::within(OTHER);
    burst.pass(&numbered(1..=300));
    burst.pause();

    let at_once = burst.pass(&numbered(1..=10));
    let at_the_pause = burst.pause();

    assert_eq!(at_once, numbered(1..=10));
    assert_eq!(at_the_pause, b"");
}

#[test]
fn holds_a_line_until_it_ends_or_the_output_pauses() {
    let mut burst = Burst::within(OTHER);

    assert_eq!(burst.pass(b"abc"), b"");
    assert_eq!(burst.pass(b"def\nghi"), b"abcdef\n");
    assert_eq!(burst.pause(), b"ghi");
    assert_eq!(burst.pause(), b"");
    assert!(!burst.has_cut());
}

#[test]
fn passes_on_a_line_that_does_not_end_within_64_kibibytes() {
    let mut burst = Burst::within(OTHER);
    let endless = vec![b'x'; 65_536];

    assert_eq!(burst.pass(&endless), b"");
    assert_eq!(burst.pass(b"y").len(), 65_537);
    assert_eq!(burst.pause(), b"");
}

#[test]
fn never_cuts_the_last_line_of_a_burst_however_long() {
    let mut burst = Burst::within(OTHER);
    let long_line = [vec![b'z'; 6_000], b"\n".to_vec()].concat();

    burst.pass(&numbered(1..=100));
    burst.pass(&long_line);
    let at_the_pause = burst.pause();

    assert_eq!(
        at_the_pause,
        [b"[+37 lines cut]\n".to_vec(), long_line].concat()
    );
}

proptest! {
    #[test]
    fn a_burst_only_loses_whole_lines_from_its_middle_and_counts_them(
        lengths in prop::collection::vec(0_usize..120, 0..300),
        cuts in prop::collection::vec(1_usize..2_000, 0..12),
    ) {
        let lines: Vec<Vec<u8>> = lengths
            .iter()
            .map(|&length| [vec![b'a'; length], b"\n".to_vec()].concat())
            .collect();
        let text = lines.concat();
        let mut burst = Burst::within(OTHER);
        let mut told = Vec::new();
        let mut rest = text.as_slice();

        for cut in cuts {
            let (piece, after) = rest.split_at(cut.min(rest.len()));
            told.extend(burst.pass(piece));
            rest = after;
        }
        told.extend(burst.pass(rest));
        told.extend(burst.pause());

        let told = String::from_utf8(told).unwrap();
        let text = String::from_utf8(text).unwrap();
        match told.split_once("[+") {
            None => prop_assert_eq!(told, text),
            Some((start, noted)) => {
                let (count, end) = noted.split_once(" lines cut]\n").unwrap();
                let cut_lines: usize = count.parse().unwrap();
                let kept = start.lines().count() + end.lines().count();

                prop_assert!(text.starts_with(start));
                prop_assert!(text.ends_with(end));
                prop_assert_eq!(cut_lines, lines.len() - kept);
                prop_assert!(text.len() > 7_000);
                prop_assert!(burst.has_cut());
            }
        }
    }
}

#[test]
fn says_it_cut_as_soon_as_lines_are_lost_and_not_before() {
    let mut burst = Burst::within(OTHER);

    burst.pass(&numbered(1..=208));
    let before_the_limit = burst.has_cut();
    burst.pass(&numbered(209..=300));

    assert!(!before_the_limit);
    assert!(burst.has_cut());
}

#[test]
fn does_not_say_it_cut_when_one_long_line_is_all_it_keeps() {
    let mut burst = Burst::within(OTHER);
    let long_line = [vec![b'z'; 6_000], b"\n".to_vec()].concat();

    burst.pass(&numbered(1..=63));
    burst.pass(&long_line);
    let at_the_pause = burst.pause();

    assert!(!burst.has_cut());
    assert_eq!(at_the_pause, long_line);
}

#[test]
fn keeps_for_the_pause_the_line_that_comes_once_the_start_is_full_to_the_byte() {
    let mut burst = Burst::within(OTHER);
    let start = [numbered(1..=62), b"eleven char\n".to_vec()].concat();

    let at_once = burst.pass(&[start.clone(), b"the next line\n".to_vec()].concat());
    let at_the_pause = burst.pause();

    assert_eq!(start.len(), 1_500);
    assert_eq!(at_once, start);
    assert_eq!(at_the_pause, b"the next line\n");
}

const HELD_BY_THE_RELAY: usize = 1_024 * 1_024;
const READ_BY_THE_RELAY: usize = 64 * 1_024;

fn count_of(line: &[u8]) -> Option<usize> {
    std::str::from_utf8(line)
        .ok()?
        .strip_prefix("[+")?
        .strip_suffix(" lines cut]\n")?
        .parse()
        .ok()
}

fn lines_cut(told: &[u8], lines: &[Vec<u8>]) -> Result<usize, String> {
    let mut next = 0;
    let mut cut = 0;
    for told_line in told.split_inclusive(|&byte| byte == b'\n') {
        if let Some(count) = count_of(told_line) {
            if count == 0 {
                return Err(format!("a count of nothing before line {next}"));
            }
            next += count;
            cut += count;
            continue;
        }
        if lines.get(next).map(Vec::as_slice) != Some(told_line) {
            let shown = String::from_utf8_lossy(&told_line[..told_line.len().min(80)]);
            return Err(format!("line {next} told as {shown:?}"));
        }
        next += 1;
    }
    if next != lines.len() {
        return Err(format!("{next} lines accounted for out of {}", lines.len()));
    }
    Ok(cut)
}

fn as_the_relay_reads(output: &[u8]) -> Vec<&[u8]> {
    let (held, rest) = output.split_at(HELD_BY_THE_RELAY.min(output.len()));
    std::iter::once(held)
        .chain(rest.chunks(READ_BY_THE_RELAY))
        .collect()
}

#[test]
fn keeps_every_line_whole_and_in_order_when_pieces_are_as_large_as_the_relay_reads() {
    let lines: Vec<Vec<u8>> = (1..=200_000)
        .map(|number| format!("line {number} of the output\n").into_bytes())
        .collect();
    let output = lines.concat();
    let mut burst = Burst::within(OTHER);
    let mut told = Vec::new();

    for piece in as_the_relay_reads(&output) {
        told.extend(burst.pass(piece));
    }
    told.extend(burst.pause());

    assert_eq!(lines_cut(&told, &lines), Ok(lines.len() - 69 - 57));
    assert!(told.starts_with(&lines[..69].concat()));
    assert!(told.ends_with(&lines[lines.len() - 57..].concat()));
}

#[test]
fn tells_what_it_kept_before_a_line_too_long_to_wait_then_the_rest_of_that_line() {
    let mut burst = Burst::within(OTHER);
    let endless = vec![b'x'; 70_000];
    burst.pass(&numbered(1..=300));

    let at_the_long_line = burst.pass(&endless);
    let in_the_middle = burst.is_mid_line();
    let its_end = burst.pass(b"y\nz\n");
    let at_the_pause = burst.pause();

    assert_eq!(
        at_the_long_line,
        [b"[+175 lines cut]\n".to_vec(), numbered(239..=300), endless].concat()
    );
    assert!(in_the_middle);
    assert_eq!(its_end, b"y\n");
    assert!(!burst.is_mid_line());
    assert_eq!(at_the_pause, b"z\n");
}

#[test]
fn passes_the_rest_of_a_line_it_began_to_tell_however_long() {
    let mut burst = Burst::within(OTHER);
    let endless = vec![b'x'; 70_000];
    burst.pass(&numbered(1..=63));

    let start = burst.pass(&endless);
    let more = burst.pass(&endless);
    let end = burst.pass(&[endless.clone(), b"\n".to_vec()].concat());

    assert_eq!(start, endless);
    assert_eq!(more, endless);
    assert_eq!(end, [endless, b"\n".to_vec()].concat());
    assert_eq!(burst.pause(), b"");
}

#[test]
fn is_mid_line_after_a_pause_that_told_a_line_not_yet_ended() {
    let mut burst = Burst::within(OTHER);

    burst.pass(b"prompt> ");
    let paused = burst.pause();
    let mid_line_at_the_pause = burst.is_mid_line();
    let answered = burst.pass(b"yes\n");

    assert_eq!(paused, b"prompt> ");
    assert!(mid_line_at_the_pause);
    assert_eq!(answered, b"yes\n");
    assert!(!burst.is_mid_line());
}

#[test]
fn starts_at_the_beginning_of_a_line() {
    assert!(!Burst::within(OTHER).is_mid_line());
}

proptest! {
    #[test]
    fn only_whole_lines_go_in_order_and_each_cut_is_counted_where_it_is_made(
        lengths in prop::collection::vec(
            prop_oneof![19 => 0_usize..120, 1 => 60_000_usize..150_000],
            0..200,
        ),
        pieces in prop::collection::vec((1_usize..300_000, any::<bool>()), 0..16),
        within_a_file in any::<bool>(),
    ) {
        let lines: Vec<Vec<u8>> = lengths
            .iter()
            .enumerate()
            .map(|(at, &length)| format!("{at}:{}\n", "a".repeat(length)).into_bytes())
            .collect();
        let output = lines.concat();
        let limits = if within_a_file { CONTENT } else { OTHER };
        let mut burst = Burst::within(limits);
        let mut told = Vec::new();
        let mut rest = output.as_slice();

        for (length, pauses) in pieces {
            let (piece, after) = rest.split_at(length.min(rest.len()));
            told.extend(burst.pass(piece));
            if pauses {
                told.extend(burst.pause());
            }
            rest = after;
        }
        told.extend(burst.pass(rest));
        told.extend(burst.pause());

        let cut = lines_cut(&told, &lines);
        prop_assert!(cut.is_ok(), "{:?}", cut);
        prop_assert_eq!(burst.has_cut(), cut.is_ok_and(|cut| cut > 0));
        prop_assert!(!burst.is_mid_line());
    }
}
