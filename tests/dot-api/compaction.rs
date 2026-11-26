use std::borrow::Cow;

use super::*;

struct Halving;

impl Compactor for Halving {
    fn compact(&self, origin: Origin<'_>, output: &str) -> Option<String> {
        let Origin::Command(line) = origin else {
            return None;
        };
        if line.starts_with(RAW_OUTPUT_MARK) {
            return None;
        }
        Some(output[..output.len() / 2].to_owned())
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        Cow::Owned(output.to_uppercase())
    }

    fn tune(&mut self, _: OutputCompaction) {}
}

#[test]
fn a_compactor_is_used_through_its_contract() {
    let compactor: &dyn Compactor = &Halving;

    assert_eq!(
        compactor.compact(Origin::Command("ls"), "abcd"),
        Some("ab".to_owned())
    );
    assert_eq!(compactor.compact(Origin::Paths, "abcd"), None);
    assert_eq!(compactor.readable("abcd"), "ABCD");
}

#[test]
fn the_raw_mark_is_a_variable_assignment_a_shell_accepts_in_front_of_a_command() {
    let (name, value) = RAW_OUTPUT_MARK.split_once('=').unwrap();

    assert!(
        name.bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
    );
    assert_eq!(value, "1");
    assert_eq!(
        Halving.compact(Origin::Command("DOT_RAW=1 cargo test"), "abcd"),
        None
    );
}

#[test]
fn the_note_of_a_cut_output_names_the_raw_mark_and_stands_on_one_line() {
    assert!(CUT_OUTPUT_NOTE.contains(RAW_OUTPUT_MARK));
    assert!(CUT_OUTPUT_NOTE.starts_with('[') && CUT_OUTPUT_NOTE.ends_with(']'));
    assert!(!CUT_OUTPUT_NOTE.contains('\n'));
}

#[test]
fn the_folder_of_whole_outputs_is_a_plain_name_a_command_line_can_carry() {
    assert!(
        WHOLE_OUTPUTS_FOLDER
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
    );
    assert_ne!(WHOLE_OUTPUTS_FOLDER, "");
}

#[derive(Default)]
struct Remembering(Option<OutputCompaction>);

impl Compactor for Remembering {
    fn compact(&self, _: Origin<'_>, _: &str) -> Option<String> {
        None
    }

    fn readable<'a>(&self, output: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(output)
    }

    fn tune(&mut self, compaction: OutputCompaction) {
        self.0 = Some(compaction);
    }
}

#[test]
fn a_compactor_is_tuned_through_its_contract() {
    let mut remembering = Remembering::default();
    let compactor: &mut dyn Compactor = &mut remembering;

    compactor.tune(OutputCompaction::FULL);

    assert_eq!(remembering.0, Some(OutputCompaction::FULL));
}
