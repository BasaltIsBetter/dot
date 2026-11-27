use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use dot_api::{
    Cancellation, Chat, Connector, Journal, Prompt, Reply, ReplyError, ReplyEvent, ReplyFailure,
    Session, Tuning, changed_lines,
};

use crate::error::Problem;
use crate::reply::asking::{Asker, Asking};
use crate::reply::recorder::{Opened, Recorder};
use crate::reply::{News, ReplyNews};
use crate::worker::{Mailbox, join};

pub(crate) struct Runner {
    jobs: Option<Sender<Job>>,
    thread: Option<JoinHandle<()>>,
}

pub(crate) enum Job {
    Reply {
        id: u64,
        prompt: Prompt,
        opened: Opened,
        cancellation: Cancellation,
        settling: Cancellation,
    },
    Reset,
    Tune(Tuning),
}

pub(crate) struct Crew<K, S, M> {
    pub(crate) connector: Arc<K>,
    pub(crate) session: Arc<S>,
    pub(crate) asking: Asking,
    pub(crate) mailbox: M,
}

impl Runner {
    pub(crate) fn start<K, S, H, M, J>(
        (chat, mut recorder): (H, Recorder<J>),
        crew: Crew<K, S, M>,
    ) -> Self
    where
        K: Connector + 'static,
        S: Session + 'static,
        H: Chat + 'static,
        M: Mailbox<ReplyNews>,
        J: Journal + 'static,
    {
        let (jobs, received) = mpsc::channel();
        let thread = thread::spawn(move || serve(chat, &mut recorder, &crew, &received));
        Self {
            jobs: Some(jobs),
            thread: Some(thread),
        }
    }

    pub(crate) fn send(&self, job: Job) {
        let Some(jobs) = &self.jobs else {
            return;
        };
        match jobs.send(job) {
            Ok(()) | Err(mpsc::SendError(_)) => {}
        }
    }

    fn stop(&mut self) {
        if let Some(thread) = self.let_go() {
            join(thread);
        }
    }

    pub(crate) fn let_go(&mut self) -> Option<JoinHandle<()>> {
        self.jobs = None;
        self.thread.take()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
}

fn serve<K, S, H, M, J>(
    mut chat: H,
    recorder: &mut Recorder<J>,
    crew: &Crew<K, S, M>,
    jobs: &Receiver<Job>,
) where
    K: Connector,
    S: Session,
    H: Chat,
    M: Mailbox<ReplyNews>,
    J: Journal,
{
    for job in jobs {
        match job {
            Job::Reset => {
                start_over(&mut chat, crew);
                recorder.started_over(Instant::now());
                recorder.remember(&chat);
            }
            Job::Tune(tuning) => chat.tune(tuning),
            Job::Reply {
                id,
                prompt,
                opened,
                cancellation,
                settling,
            } => {
                recorder.asked(&prompt, opened, Instant::now());
                let (ended, spent) = answer(&mut chat, crew, recorder, (id, prompt), &cancellation);
                recorder.ended(&ended, spent, Instant::now());
                let ended_well = ended.is_ok();
                crew.mailbox.deliver(ReplyNews {
                    id,
                    news: News::Ended(ended),
                });
                if ended_well {
                    settle(&mut chat, &settling);
                }
                recorder.remember(&chat);
            }
        }
    }
}

fn answer<K, S, H, M, J>(
    chat: &mut H,
    crew: &Crew<K, S, M>,
    recorder: &mut Recorder<J>,
    (id, prompt): (u64, Prompt),
    cancellation: &Cancellation,
) -> (Result<Reply, Problem>, Reply)
where
    K: Connector,
    S: Session,
    H: Chat,
    M: Mailbox<ReplyNews>,
    J: Journal,
{
    if cancellation.is_cancelled() {
        let stopped = Problem::plain(ReplyFailure::Stopped.text());
        return (Err(stopped), Reply::default());
    }
    let deliver = |news: ReplyNews| {
        if !cancellation.is_cancelled() {
            crew.mailbox.deliver(news);
        }
    };
    let asker = Asker {
        id,
        asking: crew.asking.clone(),
        deliver: &deliver,
    };

    let mut spent = Reply::default();
    let replied = panic::catch_unwind(AssertUnwindSafe(|| {
        let credential = crew
            .connector
            .credential(cancellation)
            .map_err(|error| Problem::new(error.failure().text(), &error))?;
        let mut on_event = |event: ReplyEvent| {
            count_changes(crew.session.as_ref(), &event);
            recorder.heard(&event, Instant::now());
            deliver(ReplyNews {
                id,
                news: News::Piece(event),
            });
        };
        let replied = chat.reply(prompt, &credential, cancellation, &asker, &mut on_event);
        spent = replied
            .as_ref()
            .map_or_else(ReplyError::spent, |reply| *reply);
        crew.session.record(&spent);
        replied.map_err(|error| Problem::new(error.failure().text(), &error))
    }));
    let ended = replied.unwrap_or_else(|panic| {
        tracing::error!("a reply crashed: the conversation starts afresh");
        start_over(chat, crew);
        Err(crashed(ReplyFailure::Crashed.text(), panic.as_ref()))
    });
    (ended, spent)
}

fn settle(chat: &mut impl Chat, settling: &Cancellation) {
    let settled = panic::catch_unwind(AssertUnwindSafe(|| chat.settle(settling)));
    if settled.is_err() {
        tracing::error!("what Dot does between two prompts crashed: the next reply does it");
    }
}

fn start_over<K, S, H: Chat, M>(chat: &mut H, crew: &Crew<K, S, M>)
where
    S: Session,
{
    let reset = panic::catch_unwind(AssertUnwindSafe(|| chat.reset()));
    debug_assert!(reset.is_ok(), "forgetting a conversation cannot fail");
    crew.session.clear_context();
}

fn count_changes(session: &impl Session, event: &ReplyEvent) {
    if let Some((added, removed)) = changed_lines(event) {
        session.count_changes(added, removed);
    }
}

fn crashed(text: dot_api::Text, panic: &(dyn Any + Send)) -> Problem {
    let message = panic
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned());
    Problem {
        text,
        cause: message,
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/reply/runner.rs"]
mod tests;
