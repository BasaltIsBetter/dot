use std::process::{Command, Stdio};
use std::time::Duration;

use dot_process::{Errors, Pace, relay};

use super::*;
use crate::PACE;

fn folder(name: &str) -> std::path::PathBuf {
    let folder =
        std::env::temp_dir().join(format!("dot-shell-relay-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&folder).unwrap_or_default();
    folder
}

fn kept_in(folder: &std::path::Path) -> WholeOutputs {
    let now = std::time::UNIX_EPOCH + Duration::from_secs(1_795_000_000);
    WholeOutputs::new(folder.to_path_buf(), now, 42)
}

fn shortening(command: &str) -> Shortening {
    let nowhere = std::path::Path::new("/dev/null/no-folder-can-be-made-here");
    Shortening::of(
        command.to_owned(),
        ShapeCompactor::default(),
        kept_in(nowhere),
    )
}

fn numbered(lines: std::ops::RangeInclusive<usize>) -> Vec<u8> {
    lines
        .map(|number| format!("line {number:04} of the output\n"))
        .collect::<Vec<String>>()
        .concat()
        .into_bytes()
}

fn file_of(lines: usize) -> String {
    (1..=lines)
        .map(|line| format!("    let value_{line:04} = compute({line});\n"))
        .collect::<Vec<String>>()
        .concat()
}

fn matches() -> String {
    (1..=20)
        .map(|line| format!("crates/dot-api/src/session.rs:{line}:    let spent = usage.cost();\n"))
        .collect::<Vec<String>>()
        .concat()
}

#[test]
fn shortens_whole_an_output_that_ended() {
    let shorter = shortening("grep -rn spent crates")
        .whole(matches().as_bytes())
        .unwrap();

    assert!(
        String::from_utf8(shorter)
            .unwrap()
            .starts_with("crates/dot-api/src/session.rs\n1:")
    );
}

#[test]
fn leaves_whole_what_is_short_or_is_not_text() {
    let mut not_text = matches().into_bytes();
    not_text.push(0xff);

    assert_eq!(shortening("grep -rn spent crates").whole(b"short"), None);
    assert_eq!(shortening("grep -rn spent crates").whole(&not_text), None);
}

#[test]
fn passes_on_as_it_is_what_no_rule_knows() {
    let mut relayed = shortening("ls -la");

    assert_eq!(
        relayed.piece(b"   Compiling a\n   Compiling b\n   Compiling c\n"),
        b"   Compiling a\n   Compiling b\n   Compiling c\n"
    );
    assert_eq!(relayed.rest(), b"");
}

#[test]
fn removes_the_noise_of_a_lasting_output_whatever_the_command() {
    let mut relayed = shortening("python3 train.py");

    assert_eq!(
        relayed.piece(b"\x1b[32mepoch 1\x1b[0m\n10%\r20%\r"),
        b"epoch 1\n"
    );
    assert_eq!(relayed.piece(b"30%\rdone\n"), b"done\n");
    assert_eq!(relayed.rest(), b"");
}

#[test]
fn counts_progress_lines_while_the_command_runs() {
    let mut relayed = shortening("cargo build");

    assert_eq!(
        relayed.piece(b"   Compiling a v1\n   Compiling b v1\n"),
        b""
    );
    assert_eq!(
        relayed.piece(b"   Compiling c v1\n    Finished in 2s\n"),
        b"[+3 lines: build progress]\n    Finished in 2s\n"
    );
    assert_eq!(relayed.rest(), b"");
}

#[test]
fn puts_back_together_a_character_cut_between_two_pieces() {
    let mut relayed = shortening("cargo build");
    let line = "warning: caf\u{e9} froid\n".as_bytes();
    let cut = line.iter().position(|&byte| byte == 0xc3).unwrap() + 1;

    assert_eq!(relayed.piece(&line[..cut]), b"");
    assert_eq!(relayed.piece(&line[cut..]), line);
}

#[test]
fn tells_what_waits_when_the_output_pauses_a_cut_character_included() {
    let mut relayed = shortening("cargo build");

    relayed.piece(b"   Compiling a v1\n   Compiling b v1\n   Compiling c v1\nd\xc3");

    assert_eq!(relayed.rest(), b"[+3 lines: build progress]\nd\xc3");
    assert_eq!(relayed.rest(), b"");
}

#[test]
fn stops_shortening_an_output_that_is_not_text() {
    let mut relayed = shortening("cargo build");

    assert_eq!(relayed.piece(b"   Compiling a v1\n"), b"");
    assert_eq!(
        relayed.piece(b"\xff\xfe binary\n"),
        b"   Compiling a v1\n\xff\xfe binary\n"
    );
    assert_eq!(
        relayed.piece(b"   Compiling b v1\n   Compiling c v1\n   Compiling d v1\n"),
        b"   Compiling b v1\n   Compiling c v1\n   Compiling d v1\n"
    );
}

#[test]
fn shortens_a_real_command_and_keeps_its_exit_status() {
    let script = "for crate in a b c d; do echo \"   Compiling $crate v1.0.0\"; done; echo 'error: no'; exit 101";
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]).stdin(Stdio::null());
    let pace = Pace {
        hold: Duration::from_secs(20),
        pause: Duration::from_secs(1),
        wait: Duration::from_secs(60),
        held: 1_024,
    };
    let mut written = Vec::new();
    let padding = "x".repeat(400);
    let mut relayed = shortening(&format!("cargo build # {padding}"));

    let ended = relay(command, Errors::Joined, pace, &mut relayed, &mut written).unwrap();

    assert_eq!(ended.status.code(), Some(101));
    assert_eq!(
        String::from_utf8(written).unwrap(),
        "   Compiling a v1.0.0\n   Compiling b v1.0.0\n   Compiling c v1.0.0\n   Compiling d v1.0.0\nerror: no\n"
    );
}

#[test]
fn shortens_a_real_command_that_lasts_line_by_line() {
    let script = "for crate in a b c d; do echo \"   Compiling $crate v1.0.0\"; done; sleep 0.5; echo '    Finished in 1s'";
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]).stdin(Stdio::null());
    let pace = Pace {
        hold: Duration::from_millis(100),
        pause: Duration::from_secs(20),
        wait: Duration::from_secs(60),
        held: 1_024,
    };
    let mut written = Vec::new();

    let ended = relay(
        command,
        Errors::Joined,
        pace,
        &mut shortening("cargo build"),
        &mut written,
    )
    .unwrap();

    assert!(ended.status.success());
    assert_eq!(
        String::from_utf8(written).unwrap(),
        "[+4 lines: build progress]\n    Finished in 1s\n"
    );
}

#[test]
fn keeps_whole_the_output_it_cut_and_names_its_file_on_the_last_line() {
    let folder = folder("cut");
    let file = file_of(400);
    let mut relayed = Shortening::of(
        "cat src/big.rs".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );

    let shorter = String::from_utf8(relayed.whole(file.as_bytes()).unwrap()).unwrap();
    let kept = folder.join("40000-42");

    assert!(shorter.len() < 6_300);
    assert!(shorter.ends_with(&format!(
        "    let value_0400 = compute(400);\n[cut: the whole output is in {}]\n",
        kept.display()
    )));
    assert!(!shorter.contains(CUT_OUTPUT_NOTE));
    assert_eq!(std::fs::read(&kept).unwrap(), file.as_bytes());
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn names_the_file_at_the_very_end_of_an_output_without_a_last_line_break() {
    let folder = folder("no-break");
    let file = file_of(400);
    let mut relayed = Shortening::of(
        "cat src/big.rs".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );

    let shorter = String::from_utf8(relayed.whole(file.trim_end().as_bytes()).unwrap()).unwrap();

    assert!(shorter.ends_with(&format!(
        "[cut: the whole output is in {}]",
        folder.join("40000-42").display()
    )));
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn says_how_to_ask_for_the_whole_output_when_it_cannot_be_kept() {
    let taken = folder("taken");
    std::fs::write(&taken, b"").unwrap();
    let mut relayed = Shortening::of(
        "cat src/big.rs".to_owned(),
        ShapeCompactor::default(),
        kept_in(&taken),
    );

    let shorter = String::from_utf8(relayed.whole(file_of(400).as_bytes()).unwrap()).unwrap();

    assert!(shorter.ends_with(&format!("{CUT_OUTPUT_NOTE}\n")));
    std::fs::remove_file(&taken).unwrap();
}

#[test]
fn keeps_no_file_of_an_output_it_shortened_without_cutting() {
    let folder = folder("uncut");
    let mut relayed = Shortening::of(
        "grep -rn spent crates".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );

    let shorter = relayed.whole(matches().as_bytes());

    assert!(shorter.is_some());
    assert!(!folder.exists());
}

#[test]
fn passes_on_as_they_come_the_contents_of_a_file_that_lasts() {
    let mut relayed = shortening("tail -f app.log");
    let piece = b"\x1b[32mgreen\x1b[0m 10%\r20%\n   Compiling a\n";

    assert_eq!(relayed.piece(piece), piece);
    assert_eq!(relayed.rest(), b"");
}

#[test]
fn cuts_a_lasting_output_burst_by_burst_and_names_its_whole_at_the_end() {
    let folder = folder("lasting");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );
    let kept = folder.join("40000-42");

    let at_once = relayed.piece(&numbered(1..=300));
    let at_the_pause = relayed.rest();
    let more = relayed.piece(b"one more line\nunfinished");
    let last = relayed.last();

    assert_eq!(at_once, numbered(1..=63));
    assert_eq!(
        String::from_utf8(at_the_pause).unwrap(),
        format!(
            "[cut: the whole output so far is in {}]\n[+175 lines cut]\n{}",
            kept.display(),
            String::from_utf8(numbered(239..=300)).unwrap()
        )
    );
    assert_eq!(more, b"one more line\n");
    assert_eq!(
        String::from_utf8(last).unwrap(),
        format!(
            "unfinished\n[cut: the whole output is in {}]\n",
            kept.display()
        )
    );
    assert_eq!(
        std::fs::read(&kept).unwrap(),
        [numbered(1..=300), b"one more line\nunfinished".to_vec()].concat()
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn forgets_the_file_of_a_lasting_output_it_did_not_cut() {
    let folder = folder("uncut-lasting");
    let mut relayed = Shortening::of(
        "./serve".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );

    let at_once = relayed.piece(&numbered(1..=20));
    let kept_meanwhile = std::fs::read_dir(&folder).unwrap().count();
    let last = relayed.last();

    assert_eq!(at_once, numbered(1..=20));
    assert_eq!(kept_meanwhile, 1);
    assert_eq!(last, b"");
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn says_how_to_ask_for_a_lasting_output_whole_when_it_cannot_be_kept() {
    let mut relayed = shortening("./serve --verbose");

    relayed.piece(&numbered(1..=300));
    let last = String::from_utf8(relayed.last()).unwrap();

    assert!(last.starts_with("[+175 lines cut]\nline 0239 of the output\n"));
    assert!(last.ends_with(&format!("line 0300 of the output\n{CUT_OUTPUT_NOTE}\n")));
}

#[test]
fn cuts_the_contents_of_a_file_that_lasts_within_the_bounds_of_a_file() {
    let mut relayed = shortening("ssh build-box cat /srv/app/big.log");

    let at_once = relayed.piece(&numbered(1..=600));
    let last = String::from_utf8(relayed.last()).unwrap();

    assert_eq!(at_once, numbered(1..=167));
    assert!(last.starts_with("[+350 lines cut]\nline 0518 of the output\n"));
    assert!(last.ends_with(&format!("{CUT_OUTPUT_NOTE}\n")));
}

#[test]
fn says_when_only_the_start_of_a_lasting_output_could_be_kept() {
    let folder = folder("too-long");
    let mut whole_outputs = kept_in(&folder);
    whole_outputs.most_kept = 1_000;
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        whole_outputs,
    );
    let kept = folder.join("40000-42");

    relayed.piece(&numbered(1..=300));
    let last = String::from_utf8(relayed.last()).unwrap();

    assert!(last.ends_with(&format!(
        "line 0300 of the output\n[cut: the first 64 MiB of the output are in {}]\n",
        kept.display()
    )));
    assert_eq!(std::fs::read(&kept).unwrap(), numbered(1..=300)[..1_000]);
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn forgets_the_file_it_can_no_longer_write() {
    let folder = folder("unwritable");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );
    relayed.piece(b"first\n");
    let kept = folder.join("40000-42");
    relayed.kept = Some(Kept {
        file: std::fs::File::open(&kept).unwrap(),
        path: kept.clone(),
        room: usize::MAX,
        is_whole: true,
    });

    relayed.piece(&numbered(1..=300));
    let last = String::from_utf8(relayed.last()).unwrap();

    assert!(!kept.exists());
    assert!(last.ends_with(&format!("line 0300 of the output\n{CUT_OUTPUT_NOTE}\n")));
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn names_the_file_of_a_lasting_output_once_before_its_end_and_only_when_it_cut() {
    let folder = folder("named-once");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );
    let note = format!(
        "[cut: the whole output so far is in {}]\n",
        folder.join("40000-42").display()
    );

    relayed.piece(&numbered(1..=20));
    let uncut = relayed.rest();
    relayed.piece(&numbered(1..=300));
    let first_cut = String::from_utf8(relayed.rest()).unwrap();
    relayed.piece(&numbered(1..=300));
    let second_cut = String::from_utf8(relayed.rest()).unwrap();

    assert_eq!(uncut, b"");
    assert!(first_cut.starts_with(&format!("{note}[+175 lines cut]\nline 0239 ")));
    assert!(first_cut.ends_with("line 0300 of the output\n"));
    assert!(second_cut.starts_with("[+175 lines cut]\nline 0239 "));
    assert!(second_cut.ends_with("line 0300 of the output\n"));
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn does_not_name_before_its_end_a_file_it_could_not_make() {
    let mut relayed = shortening("./serve --verbose");

    relayed.piece(&numbered(1..=300));
    let at_the_pause = relayed.rest();

    assert_eq!(
        at_the_pause,
        [b"[+175 lines cut]\n".to_vec(), numbered(239..=300)].concat()
    );
}

#[test]
fn says_before_its_end_too_when_only_the_start_of_the_output_is_kept() {
    let folder = folder("too-long-so-far");
    let mut whole_outputs = kept_in(&folder);
    whole_outputs.most_kept = 1_000;
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        whole_outputs,
    );

    relayed.piece(&numbered(1..=300));
    let at_the_pause = String::from_utf8(relayed.rest()).unwrap();

    assert!(at_the_pause.starts_with(&format!(
        "[cut: the first 64 MiB of the output are in {}]\n[+175 lines cut]\n",
        folder.join("40000-42").display()
    )));
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn names_the_file_before_the_line_the_output_has_not_finished() {
    let folder = folder("mid-line");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );

    relayed.piece(&numbered(1..=300));
    relayed.piece(b"still writing this li");
    let at_the_pause = String::from_utf8(relayed.rest()).unwrap();
    let then = relayed.piece(b"ne\n");

    assert!(at_the_pause.starts_with("[cut: the whole output so far is in "));
    assert!(at_the_pause.ends_with("line 0300 of the output\nstill writing this li"));
    assert_eq!(then, b"ne\n");
    std::fs::remove_dir_all(&folder).unwrap();
}

fn counted(line: &str) -> Option<usize> {
    line.strip_prefix("[+")?
        .strip_suffix(" lines cut]")?
        .parse()
        .ok()
}

#[test]
fn relays_a_long_fast_output_in_order_and_counts_each_cut_where_it_is_made() {
    let mut command = Command::new("/usr/bin/seq");
    command.args(["1", "200000"]).stdin(Stdio::null());
    let mut written = Vec::new();

    let ended = relay(
        command,
        Errors::Joined,
        PACE,
        &mut shortening("seq 1 200000"),
        &mut written,
    )
    .unwrap();

    let told = String::from_utf8(written).unwrap();
    let (lines, last) = told.trim_end().rsplit_once('\n').unwrap();
    let mut next = 1;
    for line in lines.lines() {
        if let Some(count) = counted(line) {
            next += count;
            continue;
        }
        assert_eq!(line, next.to_string());
        next += 1;
    }
    assert!(ended.status.success());
    assert_eq!(next, 200_001);
    assert_eq!(last, CUT_OUTPUT_NOTE);
    assert!(told.len() < 4_000, "{} bytes told", told.len());
}

#[test]
fn never_glues_a_note_to_a_line_it_has_not_finished() {
    let folder = folder("long-line");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );
    let endless = "x".repeat(70_000);

    relayed.piece(&numbered(1..=300));
    let at_the_long_line = String::from_utf8(relayed.piece(endless.as_bytes())).unwrap();
    let at_the_pause = relayed.rest();
    let last = String::from_utf8(relayed.last()).unwrap();

    assert!(at_the_long_line.starts_with("[+175 lines cut]\nline 0239 "));
    assert!(at_the_long_line.ends_with(&format!("line 0300 of the output\n{endless}")));
    assert_eq!(at_the_pause, b"");
    assert_eq!(
        last,
        format!(
            "\n[cut: the whole output is in {}]\n",
            folder.join("40000-42").display()
        )
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn names_the_file_so_far_once_a_line_it_began_has_ended() {
    let folder = folder("long-line-then-pause");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        ShapeCompactor::default(),
        kept_in(&folder),
    );

    relayed.piece(&numbered(1..=300));
    relayed.piece("x".repeat(70_000).as_bytes());
    let its_end = relayed.piece(b"y\n");
    let at_the_pause = String::from_utf8(relayed.rest()).unwrap();

    assert_eq!(its_end, b"y\n");
    assert_eq!(
        at_the_pause,
        format!(
            "[cut: the whole output so far is in {}]\n",
            folder.join("40000-42").display()
        )
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

fn never_cutting() -> ShapeCompactor {
    ShapeCompactor::new(dot_api::OutputCompaction {
        contents_cut: dot_api::CutLevel::Never,
        outputs_cut: dot_api::CutLevel::Never,
        ..dot_api::OutputCompaction::FULL
    })
}

#[test]
fn passes_a_lasting_output_whole_and_keeps_no_file_when_nothing_is_ever_cut() {
    let folder = folder("never-cut");
    let mut relayed = Shortening::of(
        "./serve --verbose".to_owned(),
        never_cutting(),
        kept_in(&folder),
    );

    let mut told = relayed.piece(&numbered(1..=300));
    told.extend(relayed.rest());
    told.extend(relayed.last());

    assert_eq!(told, numbered(1..=300));
    assert!(!folder.exists());
}

#[test]
fn keeps_a_finished_file_whole_when_its_cut_is_never() {
    let folder = folder("never-cut-file");
    let mut relayed = Shortening::of(
        "cat src/big.rs".to_owned(),
        never_cutting(),
        kept_in(&folder),
    );

    assert_eq!(relayed.whole(file_of(400).as_bytes()), None);
    assert!(!folder.exists());
}

#[test]
fn passes_a_lasting_output_as_it_comes_with_every_part_off() {
    let nothing = ShapeCompactor::new(dot_api::OutputCompaction {
        cleaning: dot_api::Toggle::Off,
        rewriting: dot_api::Toggle::Off,
        progress: dot_api::Toggle::Off,
        contents_cut: dot_api::CutLevel::Never,
        outputs_cut: dot_api::CutLevel::Never,
    });
    let mut relayed = Shortening::of(
        "cargo build".to_owned(),
        nothing,
        kept_in(std::path::Path::new("/dev/null/x")),
    );
    let piece = b"\x1b[32m   Compiling\x1b[0m a\r   Compiling b\n";

    assert_eq!(relayed.piece(piece), piece);
    assert_eq!(relayed.rest(), b"");
    assert_eq!(relayed.last(), b"");
}
