use proptest::prelude::*;

use super::*;

const FULL_VARIABLE: &str =
    "cleaning=on,rewriting=on,progress=on,contents=measured,outputs=measured";

#[test]
fn cuts_contents_at_the_three_bounds_of_the_study() {
    let bounds: Vec<_> = CutLevel::ALL
        .iter()
        .map(|level| {
            level
                .of_contents()
                .map(|bounds| (bounds.above, bounds.kept))
        })
        .collect();

    assert_eq!(
        bounds,
        [
            None,
            Some((13_000, 9_000)),
            Some((10_000, 6_000)),
            Some((8_000, 4_000)),
        ]
    );
}

#[test]
fn cuts_other_outputs_at_the_three_bounds_of_the_study() {
    let bounds: Vec<_> = CutLevel::ALL
        .iter()
        .map(|level| level.of_outputs().map(|bounds| (bounds.above, bounds.kept)))
        .collect();

    assert_eq!(
        bounds,
        [
            None,
            Some((9_000, 5_000)),
            Some((7_000, 3_000)),
            Some((6_000, 2_000)),
        ]
    );
}

#[test]
fn keeps_less_of_an_output_than_it_cuts_it_above() {
    for level in CutLevel::ALL {
        for bounds in [level.of_contents(), level.of_outputs()]
            .into_iter()
            .flatten()
        {
            assert!(bounds.kept < bounds.above, "{level:?}");
        }
    }
}

#[test]
fn full_compaction_does_everything_at_the_measured_bounds() {
    assert_eq!(
        OutputCompaction::FULL,
        OutputCompaction {
            cleaning: Toggle::On,
            rewriting: Toggle::On,
            progress: Toggle::On,
            contents_cut: CutLevel::Measured,
            outputs_cut: CutLevel::Measured,
        }
    );
}

#[test]
fn writes_each_choice_in_the_variable() {
    assert_eq!(OutputCompaction::FULL.to_variable(), FULL_VARIABLE);

    let quiet = OutputCompaction {
        cleaning: Toggle::Off,
        rewriting: Toggle::On,
        progress: Toggle::Off,
        contents_cut: CutLevel::Never,
        outputs_cut: CutLevel::Early,
    };
    assert_eq!(
        quiet.to_variable(),
        "cleaning=off,rewriting=on,progress=off,contents=never,outputs=early"
    );
}

#[test]
fn reads_the_choices_in_any_order() {
    let read = OutputCompaction::from_variable(
        "outputs=late,progress=off,contents=early,cleaning=on,rewriting=off",
    );

    assert_eq!(
        read,
        Some(OutputCompaction {
            cleaning: Toggle::On,
            rewriting: Toggle::Off,
            progress: Toggle::Off,
            contents_cut: CutLevel::Early,
            outputs_cut: CutLevel::Late,
        })
    );
}

#[test]
fn refuses_a_variable_that_misses_repeats_or_misnames_a_choice() {
    for value in [
        "",
        "cleaning=on,rewriting=on,progress=on,contents=measured",
        "cleaning=on,rewriting=on,progress=on,contents=measured,outputs=measured,cleaning=on",
        "cleaning=on,rewriting=on,progress=on,contents=measured,outputs=measured,sound=on",
        "cleaning=yes,rewriting=on,progress=on,contents=measured,outputs=measured",
        "cleaning=on,rewriting=on,progress=on,contents=10000,outputs=measured",
        "cleaning=on,rewriting=on,progress=on,contents=measured,outputs=Measured",
        "cleaning=on;rewriting=on;progress=on;contents=measured;outputs=measured",
        "cleaning on,rewriting=on,progress=on,contents=measured,outputs=measured",
        " cleaning=on,rewriting=on,progress=on,contents=measured,outputs=measured",
    ] {
        assert_eq!(OutputCompaction::from_variable(value), None, "{value}");
    }
}

#[test]
fn names_the_variable_like_dot_s_other_variables() {
    assert_eq!(COMPACTION_VARIABLE, "DOT_COMPACTION");
}

fn any_compaction() -> impl Strategy<Value = OutputCompaction> {
    let toggle = || prop::sample::select(vec![Toggle::On, Toggle::Off]);
    let level = || prop::sample::select(CutLevel::ALL.to_vec());
    (toggle(), toggle(), toggle(), level(), level()).prop_map(
        |(cleaning, rewriting, progress, contents_cut, outputs_cut)| OutputCompaction {
            cleaning,
            rewriting,
            progress,
            contents_cut,
            outputs_cut,
        },
    )
}

proptest! {
    #[test]
    fn any_compaction_survives_the_variable(compaction in any_compaction()) {
        prop_assert_eq!(
            OutputCompaction::from_variable(&compaction.to_variable()),
            Some(compaction)
        );
    }

    #[test]
    fn never_panics_on_any_variable(value in ".{0,120}") {
        let _ = OutputCompaction::from_variable(&value);
    }

    #[test]
    fn a_variable_read_back_writes_itself(value in "[a-z=,]{0,90}") {
        if let Some(compaction) = OutputCompaction::from_variable(&value) {
            prop_assert_eq!(
                OutputCompaction::from_variable(&compaction.to_variable()),
                Some(compaction)
            );
        }
    }
}
