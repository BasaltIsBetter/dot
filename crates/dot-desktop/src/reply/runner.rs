use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use dot_api::{
    Cancellation, Chat, Connector, Detail, Prompt, Reply, ReplyError, ReplyEvent, ReplyFailure,
    Session, ToolEnd, Tuning,
};

use crate::error::Problem;
use crate::reply::asking::{Asker, Asking};
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
    pub(crate) fn start<K, S, H, M>(chat: H, crew: Crew<K, S, M>) -> Self
    where
        K: Connector + 'static,
        S: Session + 'static,
        H: Chat + 'static,
        M: Mailbox<ReplyNews>,
    {
        let (jobs, received) = mpsc::channel();
        let thread = thread::spawn(move || serve(chat, &crew, &received));
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

fn serve<K, S, H, M>(mut chat: H, crew: &Crew<K, S, M>, jobs: &Receiver<Job>)
where
    K: Connector,
    S: Session,
    H: Chat,
    M: Mailbox<ReplyNews>,
{
    for job in jobs {
        match job {
            Job::Reset => start_over(&mut chat, crew),
            Job::Tune(tuning) => chat.tune(tuning),
            Job::Reply {
                id,
                prompt,
                cancellation,
                settling,
            } => {
                let ended = answer(&mut chat, crew, id, prompt, &cancellation);
                let ended_well = ended.is_ok();
                crew.mailbox.deliver(ReplyNews {
                    id,
                    news: News::Ended(ended),
                });
                if ended_well {
                    settle(&mut chat, &settling);
                }
            }
        }
    }
}

fn answer<K, S, H, M>(
    chat: &mut H,
    crew: &Crew<K, S, M>,
    id: u64,
    prompt: Prompt,
    cancellation: &Cancellation,
) -> Result<Reply, Problem>
where
    K: Connector,
    S: Session,
    H: Chat,
    M: Mailbox<ReplyNews>,
{
    if cancellation.is_cancelled() {
        return Err(Problem::plain(ReplyFailure::Stopped.text()));
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

    let replied = panic::catch_unwind(AssertUnwindSafe(|| {
        let credential = crew
            .connector
            .credential(cancellation)
            .map_err(|error| Problem::new(error.failure().text(), &error))?;
        let mut on_event = |event: ReplyEvent| {
            count_changes(crew.session.as_ref(), &event);
            deliver(ReplyNews {
                id,
                news: News::Piece(event),
            });
        };
        let replied = chat.reply(prompt, &credential, cancellation, &asker, &mut on_event);
        let spent = replied
            .as_ref()
            .map_or_else(ReplyError::spent, |reply| *reply);
        crew.session.record(&spent);
        replied.map_err(|error| Problem::new(error.failure().text(), &error))
    }));
    replied.unwrap_or_else(|panic| {
        tracing::error!("a reply crashed: the conversation starts afresh");
        start_over(chat, crew);
        Err(crashed(ReplyFailure::Crashed.text(), panic.as_ref()))
    })
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
    if let ReplyEvent::ToolEnded {
        end: ToolEnd::Done(Detail::Changes { added, removed }),
        ..
    } = event
    {
        session.count_changes(*added, *removed);
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
