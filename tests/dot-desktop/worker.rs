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

    workers.spawn(|_| 7_u32);

    assert_eq!(receiver.recv_timeout(PATIENCE), Ok(7));
}

#[test]
fn a_cancelled_task_delivers_nothing() {
    let (mut workers, receiver) = workers();

    workers.spawn(until_cancelled).cancel();
    workers.stop_all();

    assert!(receiver.try_recv().is_err());
}

#[test]
fn stopping_cancels_and_waits_for_every_task() {
    let (mut workers, receiver) = workers();
    workers.spawn(until_cancelled);
    workers.spawn(until_cancelled);

    workers.stop_all();

    assert!(workers.running.is_empty());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn dropping_the_workers_stops_their_tasks() {
    let (mut workers, _receiver) = workers();
    let stopped = Arc::new(AtomicBool::new(false));
    let seen_by_task = Arc::clone(&stopped);
    workers.spawn(move |cancellation| {
        let value = until_cancelled(cancellation);
        seen_by_task.store(true, Ordering::Relaxed);
        value
    });

    drop(workers);

    assert!(stopped.load(Ordering::Relaxed));
}

#[test]
fn forgets_the_tasks_that_have_finished() {
    let (mut workers, receiver) = workers();
    workers.spawn(|_| 1_u32);
    receiver.recv_timeout(PATIENCE).unwrap();
    while !workers.running[0].handle.is_finished() {
        thread::sleep(Duration::from_millis(1));
    }

    workers.spawn(until_cancelled);

    assert_eq!(workers.running.len(), 1);
}

#[test]
fn a_streaming_task_delivers_each_piece_then_its_end() {
    let (mut workers, receiver) = workers();

    workers.spawn_streaming(|_, deliver| {
        deliver(1_u32);
        deliver(2);
        3
    });

    let delivered: Vec<u32> = (0..3)
        .map(|_| receiver.recv_timeout(PATIENCE).unwrap())
        .collect();
    assert_eq!(delivered, [1, 2, 3]);
}

#[test]
fn a_cancelled_streaming_task_delivers_nothing_more() {
    let (mut workers, receiver) = workers();
    let (started_sender, started) = mpsc::channel();

    let cancellation = workers.spawn_streaming(move |cancellation, deliver| {
        deliver(1_u32);
        started_sender.send(()).unwrap();
        until_cancelled(cancellation);
        deliver(2);
        3
    });
    started.recv_timeout(PATIENCE).unwrap();
    cancellation.cancel();
    workers.stop_all();

    assert_eq!(receiver.recv_timeout(PATIENCE), Ok(1));
    assert!(receiver.try_recv().is_err());
}
