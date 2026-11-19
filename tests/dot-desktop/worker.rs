use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::time::Duration;

use super::*;

const PATIENCE: Duration = Duration::from_secs(5);

impl<E: Send + 'static, T: Into<E>> Mailbox<T> for Sender<E> {
    fn deliver(&self, event: T) {
        match self.send(event.into()) {
            Ok(()) | Err(SendError(_)) => {}
        }
    }
}

fn workers() -> (Workers<Sender<u32>>, Receiver<u32>) {
    let (sender, receiver) = mpsc::channel();
    (Workers::new(sender), receiver)
}

fn until_cancelled(cancellation: &Cancellation) -> u32 {
    while !cancellation.is_cancelled() {
        thread::sleep(Duration::from_millis(1));
    }
    1
}

#[test]
fn delivers_what_the_task_returns() {
    let (mut workers, receiver) = workers();

    workers.spawn(|_| 7_u32, || None);

    assert_eq!(receiver.recv_timeout(PATIENCE), Ok(7));
}

#[test]
fn a_cancelled_task_delivers_nothing() {
    let (mut workers, receiver) = workers();

    workers.spawn(until_cancelled, || None).cancel();
    workers.stop_all();

    assert!(receiver.try_recv().is_err());
}

#[test]
fn stopping_cancels_and_waits_for_every_task() {
    let (mut workers, receiver) = workers();
    workers.spawn(until_cancelled, || None);
    workers.spawn(until_cancelled, || None);

    workers.stop_all();

    assert!(workers.running.is_empty());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn dropping_the_workers_stops_their_tasks() {
    let (mut workers, _receiver) = workers();
    let stopped = Arc::new(AtomicBool::new(false));
    let seen_by_task = Arc::clone(&stopped);
    workers.spawn(
        move |cancellation| {
            let value = until_cancelled(cancellation);
            seen_by_task.store(true, Ordering::Relaxed);
            value
        },
        || None,
    );

    drop(workers);

    assert!(stopped.load(Ordering::Relaxed));
}

#[test]
fn forgets_the_tasks_that_have_finished() {
    let (mut workers, receiver) = workers();
    workers.spawn(|_| 1_u32, || None);
    receiver.recv_timeout(PATIENCE).unwrap();
    while !workers.running[0].handle.is_finished() {
        thread::sleep(Duration::from_millis(1));
    }

    workers.spawn(until_cancelled, || None);

    assert_eq!(workers.running.len(), 1);
}

#[test]
fn a_task_that_panics_delivers_what_stands_for_it() {
    let (mut workers, receiver) = workers();

    workers.spawn(|_| -> u32 { panic!("the task failed") }, || Some(9));

    assert_eq!(receiver.recv_timeout(PATIENCE), Ok(9));
    workers.stop_all();
}

#[test]
fn a_task_that_panics_without_stand_in_delivers_nothing_and_stops_cleanly() {
    let (mut workers, receiver) = workers();

    workers.spawn(|_| -> u32 { panic!("the task failed") }, || None);
    workers.stop_all();

    assert!(workers.running.is_empty());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn a_panic_never_reaches_the_next_task() {
    let (mut workers, receiver) = workers();
    workers.spawn(|_| -> u32 { panic!("the task failed") }, || None);
    while !workers.running[0].handle.is_finished() {
        thread::sleep(Duration::from_millis(1));
    }

    workers.spawn(|_| 4_u32, || None);

    assert_eq!(receiver.recv_timeout(PATIENCE), Ok(4));
}

#[test]
fn a_cancelled_task_that_panics_delivers_nothing() {
    let (mut workers, receiver) = workers();

    workers
        .spawn(
            |cancellation| -> u32 {
                until_cancelled(cancellation);
                panic!("the task failed")
            },
            || Some(9),
        )
        .cancel();
    workers.stop_all();

    assert!(receiver.try_recv().is_err());
}
