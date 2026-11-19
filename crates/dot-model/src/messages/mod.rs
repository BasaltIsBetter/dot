mod request;
mod response;
mod tools;

use std::sync::{Mutex, PoisonError};

use dot_api::{
    Cancellation, Effort, Model, Permissions, Prompt, Reply, ReplyError, ReplyEvent, ReplyFailure,
    Toolbox,
};
use ureq::Agent;
use ureq::http::Response;

use crate::stream::Turn;
use crate::tool::system_prompt;

pub(crate) use self::request::Authorization;
use self::tools::{Spent, answer};

pub(crate) struct MessagesChat {
    url: String,
    model: Model,
    effort: Effort,
    history: Mutex<Vec<u8>>,
}

pub(crate) struct Hands<'a, T> {
    pub(crate) toolbox: &'a T,
    pub(crate) permissions: &'a dyn Permissions,
    pub(crate) cancellation: &'a Cancellation,
}

impl MessagesChat {
    pub(crate) fn new(model: Model, effort: Effort) -> Self {
        Self::at(request::MESSAGES_URL, model, effort)
    }

    pub(crate) fn at(url: impl Into<String>, model: Model, effort: Effort) -> Self {
        Self {
            url: url.into(),
            model,
            effort,
            history: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn reply<T: Toolbox>(
        &self,
        prompt: &Prompt,
        authorization: Authorization<'_>,
        hands: &Hands<'_, T>,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        let cancellation = hands.cancellation;
        let folder = hands.toolbox.folder();
        let system = system_prompt(folder);
        let mut history = self.history.lock().unwrap_or_else(PoisonError::into_inner);
        request::append_user(&mut history, prompt).map_err(rejected)?;

        let agent = request::agent();
        let interrupted = agent.clone();
        cancellation.on_cancel(move || interrupted.shutdown());
        let mut spent = Spent::default();

        loop {
            let body =
                request::body(self.model.id, self.effort, &system, &history).map_err(rejected)?;
            let response = self.send(&agent, authorization, &body, cancellation)?;
            let mut turn = Turn::replayable(folder);
            let streamed = response::read_stream(
                response.into_body().into_reader(),
                &mut turn,
                on_event,
                cancellation,
            );
            if turn.is_declined() {
                return Err(ReplyError::new(ReplyFailure::Declined));
            }
            request::append_assistant(&mut history, &turn.replayed()).map_err(rejected)?;
            spent.add(&turn);

            let runs = streamed.is_ok() && turn.wants_tools();
            let results = answer(&turn, runs, hands, on_event);
            request::append_tool_results(&mut history, &results).map_err(rejected)?;
            streamed?;
            if cancellation.is_cancelled() {
                return Err(ReplyError::new(ReplyFailure::Stopped));
            }
            if !turn.wants_tools() && !turn.is_paused() {
                return Ok(spent.reply());
            }
        }
    }

    fn send(
        &self,
        agent: &Agent,
        authorization: Authorization<'_>,
        body: &[u8],
        cancellation: &Cancellation,
    ) -> Result<Response<ureq::Body>, ReplyError> {
        let mut attempt = 0;

        loop {
            let sent = request::post(agent, &self.url, authorization, body);
            if cancellation.is_cancelled() {
                return Err(ReplyError::new(ReplyFailure::Stopped));
            }
            let (status, retry_after, failure) = match sent {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => {
                    let status = response.status().as_u16();
                    let retry_after = response
                        .headers()
                        .get(response::RETRY_AFTER)
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_owned);
                    (Some(status), retry_after, response::status_error(response))
                }
                Err(error) => (
                    None,
                    None,
                    ReplyError::caused_by(ReplyFailure::Unreachable, error),
                ),
            };

            let Some(delay) = response::retry_delay(status, retry_after.as_deref(), attempt) else {
                return Err(failure);
            };
            if !response::pause(delay, cancellation) {
                return Err(ReplyError::new(ReplyFailure::Stopped));
            }
            attempt += 1;
        }
    }
}

fn rejected(error: serde_json::Error) -> ReplyError {
    ReplyError::caused_by(ReplyFailure::Rejected, error)
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/messages/mod.rs"]
mod tests;
