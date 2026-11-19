use std::sync::atomic::AtomicU32;
use std::thread;

use super::*;

fn counter() -> (Arc<AtomicU32>, impl Fn() + Send + 'static) {
    let count = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&count);
    (count, move || {
        counted.fetch_add(1, Ordering::Relaxed);
    })
}

#[test]
fn a_cancellation_starts_lowered() {
    assert!(!Cancellation::new().is_cancelled());
}

#[test]
fn cancelling_raises_the_flag_for_every_clone() {
    let cancellation = Cancellation::new();
    let handed_to_the_task = cancellation.clone();

    cancellation.cancel();

    assert!(handed_to_the_task.is_cancelled());
}

#[test]
fn runs_a_registered_action_when_cancelled_and_not_before() {
    let cancellation = Cancellation::new();
    let (count, action) = counter();

    let _guard = cancellation.on_cancel(action);
    assert_eq!(count.load(Ordering::Relaxed), 0);

    cancellation.clone().cancel();
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

#[test]
fn runs_each_action_once_however_often_cancelled() {
    let cancellation = Cancellation::new();
    let (count, action) = counter();
    let _guard = cancellation.on_cancel(action);

    cancellation.cancel();
    cancellation.cancel();

    assert_eq!(count.load(Ordering::Relaxed), 1);
}

#[test]
fn runs_an_action_at_once_when_already_cancelled() {
    let cancellation = Cancellation::new();
    cancellation.cancel();
    let (count, action) = counter();

    let guard = cancellation.on_cancel(action);

    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert_eq!(
        format!("{guard:?}"),
        "CancelGuard { registered: false, .. }"
    );
}

#[test]
fn runs_every_registered_action() {
    let cancellation = Cancellation::new();
    let (first, first_action) = counter();
    let (second, second_action) = counter();
    let _first = cancellation.on_cancel(first_action);
    let _second = cancellation.on_cancel(second_action);

    cancellation.cancel();

    assert_eq!(first.load(Ordering::Relaxed), 1);
    assert_eq!(second.load(Ordering::Relaxed), 1);
}

#[test]
fn wakes_a_task_from_another_thread() {
    let cancellation = Cancellation::new();
    let (count, action) = counter();
    let _guard = cancellation.on_cancel(action);
    let canceller = cancellation.clone();

    thread::spawn(move || canceller.cancel()).join().unwrap();

    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert!(cancellation.is_cancelled());
}

#[test]
fn shows_whether_it_is_raised_when_debugged() {
    let cancellation = Cancellation::new();
    assert_eq!(format!("{cancellation:?}"), "Cancellation(false)");

    cancellation.cancel();
    assert_eq!(format!("{cancellation:?}"), "Cancellation(true)");
}

#[test]
fn a_dropped_guard_keeps_its_action_from_running() {
    let cancellation = Cancellation::new();
    let (count, action) = counter();
    let (kept, kept_action) = counter();
    let _kept = cancellation.on_cancel(kept_action);

    drop(cancellation.on_cancel(action));
    cancellation.cancel();

    assert_eq!(count.load(Ordering::Relaxed), 0);
    assert_eq!(kept.load(Ordering::Relaxed), 1);
}

#[test]
fn a_dropped_guard_releases_what_its_action_owns() {
    let cancellation = Cancellation::new();
    let owned = Arc::new(());
    let held = Arc::clone(&owned);

    let guard = cancellation.on_cancel(move || drop(held));
    assert_eq!(Arc::strong_count(&owned), 2);
    drop(guard);

    assert_eq!(Arc::strong_count(&owned), 1);
}

#[test]
fn a_guard_dropped_after_cancelling_does_nothing() {
    let cancellation = Cancellation::new();
    let (count, action) = counter();
    let guard = cancellation.on_cancel(action);
    assert_eq!(format!("{guard:?}"), "CancelGuard { registered: true, .. }");

    cancellation.cancel();
    drop(guard);

    assert_eq!(count.load(Ordering::Relaxed), 1);
}
