use std::io::{self, BufReader, Read as _};
use std::thread;
use std::time::{Duration, Instant};

use dot_api::{Cancellation, ReplyError, ReplyEvent, ReplyFailure};
use serde::Deserialize;
use ureq::Body;
use ureq::http::Response;

use crate::line::{Read, read_line};
use crate::stream::{ApiError, Event, Turn};

const MEBIBYTE: usize = 1_024 * 1_024;
const LONGEST_LINE: usize = 4 * MEBIBYTE;
const LONGEST_ERROR_BODY: u64 = 64 * 1_024;
const DATA_PREFIX: &[u8] = b"data:";
const RETRIES: u32 = 2;
const FIRST_RETRY_DELAY: Duration = Duration::from_millis(500);
const LONGEST_RETRY_DELAY: Duration = Duration::from_secs(20);
const PAUSE_STEP: Duration = Duration::from_millis(50);
pub(crate) const RETRY_AFTER: &str = "retry-after";

#[derive(Debug, thiserror::Error)]
#[error("Anthropic answered with HTTP status {status}")]
struct StatusError {
    status: u16,
    #[source]
    detail: Option<ApiError>,
}

#[derive(Debug, thiserror::Error)]
#[error("the reply stream ended before Claude finished")]
struct CutShort;

#[derive(Debug, thiserror::Error)]
#[error("a line of the reply stream is longer than {} MiB", LONGEST_LINE / MEBIBYTE)]
struct LineTooLong;

#[derive(Deserialize)]
struct ErrorBody {
    error: ApiError,
}

pub(crate) fn read_stream(
    body: impl io::Read,
    turn: &mut Turn,
    on_event: &mut dyn FnMut(ReplyEvent),
    cancellation: &Cancellation,
) -> Result<(), ReplyError> {
    let mut reader = BufReader::new(body);
    let mut line = Vec::new();

    loop {
        let read = read_line(&mut reader, &mut line, LONGEST_LINE)
            .map_err(|error| cut(error, cancellation))?;
        match read {
            Read::End => break,
            Read::TooLong => {
                turn.discard();
                return Err(ReplyError::caused_by(ReplyFailure::Oversized, LineTooLong));
            }
            Read::Line => {}
        }
        let Some(data) = line.strip_prefix(DATA_PREFIX) else {
            continue;
        };
        let event: Event = serde_json::from_slice(data)
            .map_err(|error| ReplyError::caused_by(ReplyFailure::Unreachable, error))?;
        if let Event::Error { error } = event {
            return Err(ReplyError::caused_by(error.failure(), error));
        }
        turn.apply(event, on_event);
    }

    if cancellation.is_cancelled() {
        return Err(ReplyError::new(ReplyFailure::Stopped));
    }
    if !turn.is_finished() {
        return Err(ReplyError::caused_by(ReplyFailure::Unreachable, CutShort));
    }
    Ok(())
}

pub(crate) fn status_error(response: Response<Body>) -> ReplyError {
    let status = response.status().as_u16();
    let mut body = Vec::new();
    let detail = response
        .into_body()
        .into_reader()
        .take(LONGEST_ERROR_BODY)
        .read_to_end(&mut body)
        .ok()
        .and_then(|_| serde_json::from_slice::<ErrorBody>(&body).ok())
        .map(|body| body.error);
    ReplyError::caused_by(failure_of_status(status), StatusError { status, detail })
}

pub(crate) const fn failure_of_status(status: u16) -> ReplyFailure {
    match status {
        401 | 403 => ReplyFailure::Unauthorized,
        429 => ReplyFailure::RateLimited,
        503 | 529 => ReplyFailure::Overloaded,
        400..=499 => ReplyFailure::Rejected,
        _ => ReplyFailure::Unreachable,
    }
}

pub(crate) fn retry_delay(
    status: Option<u16>,
    retry_after: Option<&str>,
    attempt: u32,
) -> Option<Duration> {
    if attempt >= RETRIES || status.is_some_and(|status| !is_retried(status)) {
        return None;
    }
    let asked = retry_after
        .and_then(|seconds| seconds.trim().parse::<u64>().ok())
        .map(Duration::from_secs);
    match asked {
        Some(asked) if asked > LONGEST_RETRY_DELAY => None,
        Some(asked) => Some(asked),
        None => Some(FIRST_RETRY_DELAY * 2_u32.pow(attempt)),
    }
}

pub(crate) fn pause(delay: Duration, cancellation: &Cancellation) -> bool {
    let until = Instant::now() + delay;
    loop {
        if cancellation.is_cancelled() {
            return false;
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        thread::sleep(left.min(PAUSE_STEP));
    }
}

const fn is_retried(status: u16) -> bool {
    matches!(status, 408 | 409 | 429 | 500..=599)
}

fn cut(error: io::Error, cancellation: &Cancellation) -> ReplyError {
    if cancellation.is_cancelled() {
        return ReplyError::new(ReplyFailure::Stopped);
    }
    ReplyError::caused_by(ReplyFailure::Unreachable, error)
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/messages/response.rs"]
pub(crate) mod tests;
