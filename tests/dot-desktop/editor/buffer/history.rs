use super::*;

fn step(at: usize, inserted: &str, removed: &str, when: Instant) -> Step {
    Step {
        at,
        removed: removed.to_owned(),
        inserted: inserted.to_owned(),
        caret: at,
        when,
    }
}

fn counted(history: &History) -> usize {
    history
        .undone
        .iter()
        .chain(&history.redone)
        .map(Step::bytes)
        .sum()
}

#[test]
fn forgets_the_oldest_steps_once_the_history_weighs_more_than_its_budget() {
    let start = Instant::now();
    let mut history = History::default();
    let mebibyte = "x".repeat(1_024 * 1_024);

    for at in 0..20 {
        history.record(step(at, &mebibyte, "", start), false);
    }

    assert_eq!(history.undone.len(), 16);
    assert_eq!(history.undone.front().map(|step| step.at), Some(4));
    assert_eq!(history.bytes, counted(&history));
    assert!(history.bytes <= MOST_UNDO_BYTES);
}

#[test]
fn keeps_the_last_step_even_when_it_alone_weighs_more_than_the_budget() {
    let start = Instant::now();
    let mut history = History::default();
    history.record(step(0, "a", "", start), false);

    history.record(step(0, &"x".repeat(MOST_UNDO_BYTES + 1), "", start), false);

    assert_eq!(history.undone.len(), 1);
    assert!(history.undo().is_some());
}

#[test]
fn keeps_five_hundred_light_steps_at_most() {
    let start = Instant::now();
    let mut history = History::default();

    for at in 0..600 {
        history.record(step(at, "a\n", "", start), false);
    }

    assert_eq!(history.undone.len(), MOST_STEPS);
    assert_eq!(history.undone.front().map(|step| step.at), Some(100));
}

#[test]
fn counts_the_same_weight_across_undo_redo_and_a_new_change() {
    let start = Instant::now();
    let mut history = History::default();
    history.record(step(0, "abc", "", start), false);
    history.record(step(3, "d", "", start), true);
    history.record(step(0, "", "abcd", start), false);

    history.undo();
    history.undo();
    let while_undone = history.bytes;
    history.redo();
    history.record(step(0, "z", "", start), false);

    assert_eq!(while_undone, 8);
    assert_eq!(history.redone.len(), 0);
    assert_eq!(history.bytes, counted(&history));
    assert_eq!(history.bytes, 5);
}
