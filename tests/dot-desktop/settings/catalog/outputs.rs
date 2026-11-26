use dot_api::Text;

use super::*;
use crate::app::tests::IdTranslator;

const EVERY_PART: [OutputPart; 5] = [
    OutputPart::Cleaning,
    OutputPart::Rewriting,
    OutputPart::Progress,
    OutputPart::ContentsCut,
    OutputPart::OutputsCut,
];

fn compaction_after(part: OutputPart, option: usize) -> OutputCompaction {
    let mut compaction = OutputCompaction::FULL;
    part.choose(option, &mut compaction);
    compaction
}

#[test]
fn turns_each_part_on_or_off() {
    assert_eq!(
        compaction_after(OutputPart::Cleaning, 1).cleaning,
        Toggle::Off
    );
    assert_eq!(
        compaction_after(OutputPart::Rewriting, 1).rewriting,
        Toggle::Off
    );
    assert_eq!(
        compaction_after(OutputPart::Progress, 1).progress,
        Toggle::Off
    );
    assert_eq!(
        compaction_after(OutputPart::Progress, 0).progress,
        Toggle::On
    );
}

#[test]
fn offers_every_cut_level_from_never_to_early() {
    let levels: Vec<CutLevel> = (0..OutputPart::ContentsCut.options())
        .map(|option| compaction_after(OutputPart::ContentsCut, option).contents_cut)
        .collect();

    assert_eq!(levels, CutLevel::ALL);
    assert_eq!(
        compaction_after(OutputPart::OutputsCut, 0).outputs_cut,
        CutLevel::Never
    );
    assert_eq!(
        compaction_after(OutputPart::OutputsCut, 3).outputs_cut,
        CutLevel::Early
    );
}

#[test]
fn changes_nothing_else_than_its_own_part() {
    let changed = compaction_after(OutputPart::ContentsCut, 0);

    assert_eq!(
        changed,
        OutputCompaction {
            contents_cut: CutLevel::Never,
            ..OutputCompaction::FULL
        }
    );
}

#[test]
fn ignores_an_option_past_the_last() {
    for part in EVERY_PART {
        assert_eq!(
            compaction_after(part, part.options()),
            OutputCompaction::FULL,
            "{part:?}"
        );
    }
}

#[test]
fn reads_back_the_option_chosen() {
    for part in EVERY_PART {
        for option in 0..part.options() {
            assert_eq!(
                part.chosen(compaction_after(part, option)),
                option,
                "{part:?} {option}"
            );
        }
    }
}

#[test]
fn names_each_cut_by_its_bounds_in_kilobytes() {
    let names = |part: OutputPart| -> Vec<String> {
        (0..=part.options())
            .map(|option| part.option_name(option, &IdTranslator))
            .collect()
    };
    let bounds = |above_and_kept: &str| format!("{} {above_and_kept}", Text::CutBounds.id());

    assert_eq!(
        names(OutputPart::ContentsCut),
        [
            Text::CutNever.id().to_owned(),
            bounds("13 → 9"),
            bounds("10 → 6"),
            bounds("8 → 4"),
            String::new(),
        ]
    );
    assert_eq!(
        names(OutputPart::OutputsCut),
        [
            Text::CutNever.id().to_owned(),
            bounds("9 → 5"),
            bounds("7 → 3"),
            bounds("6 → 2"),
            String::new(),
        ]
    );
}

#[test]
fn names_a_toggle_yes_or_no() {
    for part in [
        OutputPart::Cleaning,
        OutputPart::Rewriting,
        OutputPart::Progress,
    ] {
        assert_eq!(part.option_name(0, &IdTranslator), Text::ToggleOn.id());
        assert_eq!(part.option_name(1, &IdTranslator), Text::ToggleOff.id());
        assert_eq!(part.option_name(2, &IdTranslator), "");
    }
}

#[test]
fn gives_each_part_its_own_label_and_hint() {
    let mut texts: Vec<&str> = EVERY_PART
        .iter()
        .flat_map(|part| [part.label().id(), part.hint().id()])
        .collect();

    texts.sort_unstable();
    texts.dedup();

    assert_eq!(texts.len(), 2 * EVERY_PART.len());
    assert_eq!(OutputPart::ContentsCut.label(), Text::SettingContentsCut);
    assert_eq!(OutputPart::Progress.hint(), Text::SettingOutputProgressHint);
}
