mod request;
mod response;
mod tools;

use std::path::Path;

use dot_api::{
    Cancellation, Compactor, Effort, Model, Permissions, Prompt, Reply, ReplyError, ReplyEvent,
    ReplyFailure, Toolbox,
};
use ureq::Agent;
use ureq::http::Response;

use crate::stream::Turn;
use crate::tool::system_prompt;

pub(crate) use self::request::Authorization;
#[cfg(feature = "bench")]
pub(crate) use self::request::head;
#[cfg(feature = "bench")]
pub(crate) use self::response::read_stream;
use self::tools::{Spent, answer};
#[cfg(feature = "bench")]
pub(crate) use crate::stream::Turn as BenchTurn;

pub(crate) struct MessagesChat {
    url: String,
    model: Model,
    effort: Effort,
    history: Vec<u8>,
}

pub(crate) struct Hands<'a, T> {
    pub(crate) toolbox: &'a mut T,
    pub(crate) permissions: &'a dyn Permissions,
    pub(crate) cancellation: &'a Cancellation,
    pub(crate) compactor: &'a dyn Compactor,
}

#[derive(Debug, Default)]
struct Exchange {
    spent: Spent,
    answered: bool,
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
            history: Vec::new(),
        }
    }

    pub(crate) fn reply<T: Toolbox>(
        &mut self,
        prompt: Prompt,
        authorization: Authorization<'_>,
        hands: &mut Hands<'_, T>,
        on_event: &mut dyn FnMut(ReplyEvent),
    ) -> Result<Reply, ReplyError> {
        let start = self.history.len();
        request::append_user(&mut self.history, &prompt).map_err(rejected)?;
        drop(prompt);

        let mut exchange = Exchange {
            spent: Spent::of_a_reply(start == 0),
            answered: false,
        };
        let ended = self.exchange(authorization, hands, on_event, &mut exchange);
        if ended.is_err() && !exchange.answered {
            self.history.truncate(start);
        }
        let spent = exchange.spent.reply();
        ended
            .map(|()| spent)
            .map_err(|error| error.with_spent(spent))
    }

    pub(crate) fn reset(&mut self) {
        self.history = Vec::new();
    }

    fn exchange<T: Toolbox>(
        &mut self,
        authorization: Authorization<'_>,
        hands: &mut Hands<'_, T>,
        on_event: &mut dyn FnMut(ReplyEvent),
        exchange: &mut Exchange,
    ) -> Result<(), ReplyError> {
        let cancellation = hands.cancellation;
        let folder = hands.toolbox.folder().map(Path::to_path_buf);
        let folder = folder.as_deref();
        let head =
            request::head(self.model.id, self.effort, &system_prompt(folder)).map_err(rejected)?;
        let agent = request::agent();
        let interrupted = agent.clone();
        let _shutdown = cancellation.on_cancel(move || interrupted.shutdown());

        loop {
            let response = self.send(&agent, authorization, &head, cancellation)?;
            let mut turn = Turn::replayable(folder);
            let streamed = response::read_stream(
                response.into_body().into_reader(),
                &mut turn,
                on_event,
                cancellation,
            );
            exchange.spent.add(&turn);
            if turn.is_declined() {
                return Err(ReplyError::new(ReplyFailure::Declined));
            }
            exchange.answered |=
                request::append_assistant(&mut self.history, &turn.replayed()).map_err(rejected)?;

            let runs = streamed.is_ok() && turn.wants_tools();
            let results = answer(&turn, runs, hands, on_event);
            request::append_tool_results(&mut self.history, &results).map_err(rejected)?;
            streamed?;
            if cancellation.is_cancelled() {
                return Err(ReplyError::new(ReplyFailure::Stopped));
            }
            if !turn.wants_tools() && !turn.is_paused() {
                return Ok(());
            }
        }
    }

    fn send(
        &self,
        agent: &Agent,
        authorization: Authorization<'_>,
        head: &[u8],
        cancellation: &Cancellation,
    ) -> Result<Response<ureq::Body>, ReplyError> {
        let mut attempt = 0;

        loop {
            let sent = request::post(agent, &self.url, authorization, head, &self.history);
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
            tracing::info!(
                status = status.unwrap_or_default(),
                attempt,
                delay_ms = delay.as_millis(),
                "Anthropic did not answer, retrying"
            );
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
pub(crate) mod tests;
