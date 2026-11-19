use std::thread::{self, JoinHandle};

use dot_api::Cancellation;
use winit::event_loop::{EventLoopClosed, EventLoopProxy};

pub(crate) trait Mailbox<E>: Clone + Send + 'static {
    fn deliver(&self, event: E);
}

pub(crate) struct Workers<M> {
    mailbox: M,
    running: Vec<Running>,
}

struct Running {
    cancellation: Cancellation,
    handle: JoinHandle<()>,
}

impl<E: Send + 'static, T: Into<E>> Mailbox<T> for EventLoopProxy<E> {
    fn deliver(&self, event: T) {
        match self.send_event(event.into()) {
            Ok(()) | Err(EventLoopClosed(_)) => {}
        }
    }
}

impl<M> Workers<M> {
    pub(crate) fn new(mailbox: M) -> Self {
        Self {
            mailbox,
            running: Vec::new(),
        }
    }

    pub(crate) fn spawn<E>(
        &mut self,
        task: impl FnOnce(&Cancellation) -> E + Send + 'static,
    ) -> Cancellation
    where
        E: Send + 'static,
        M: Mailbox<E>,
    {
        self.spawn_streaming(move |cancellation, _| task(cancellation))
    }

    pub(crate) fn spawn_streaming<E>(
        &mut self,
        task: impl FnOnce(&Cancellation, &dyn Fn(E)) -> E + Send + 'static,
    ) -> Cancellation
    where
        E: Send + 'static,
        M: Mailbox<E>,
    {
        self.join_finished();

        let cancellation = Cancellation::new();
        let handed = cancellation.clone();
        let mailbox = self.mailbox.clone();
        let handle = thread::spawn(move || {
            let deliver = |event: E| {
                if !handed.is_cancelled() {
                    mailbox.deliver(event);
                }
            };
            let last = task(&handed, &deliver);
            deliver(last);
        });

        self.running.push(Running {
            cancellation: cancellation.clone(),
            handle,
        });
        cancellation
    }

    pub(crate) fn stop_all(&mut self) {
        for running in self.running.drain(..) {
            running.cancellation.cancel();
            join(running.handle);
        }
    }

    fn join_finished(&mut self) {
        let (finished, running) = self
            .running
            .drain(..)
            .partition::<Vec<_>, _>(|running| running.handle.is_finished());
        self.running = running;
        for finished in finished {
            join(finished.handle);
        }
    }
}

impl<M> Drop for Workers<M> {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn join(handle: JoinHandle<()>) {
    if let Err(panic) = handle.join() {
        std::panic::resume_unwind(panic);
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-desktop/worker.rs"]
pub(crate) mod tests;
