use std::io::{self, Read};
use std::process::{Child, ExitStatus};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dot_api::Cancellation;
use dot_process::Waited;

use crate::engine::EngineError;

const LONGEST_OUTPUT: u64 = 1_024 * 1_024;
const LONGEST_REFUSAL: usize = 2_000;
const READER_PANICKED: &str = "reading what the engine printed failed";

pub(crate) struct Ended {
    status: ExitStatus,
    output: String,
    errors: String,
}

impl Ended {
    pub(crate) fn printed(self) -> Result<String, EngineError> {
        if self.status.success() {
            return Ok(self.output);
        }
        Err(EngineError::Refused(refusal(
            &self.errors,
            &self.output,
            self.status,
        )))
    }
}

pub(crate) fn run(
    mut child: Child,
    timeout: Duration,
    cancellation: &Cancellation,
) -> Result<Ended, EngineError> {
    let output = child.stdout.take().map(listen);
    let errors = child.stderr.take().map(listen);
    let deadline = Instant::now() + timeout;
    let waited =
        dot_process::wait(&mut child, Some(deadline), cancellation).map_err(EngineError::Lost)?;
    let status = match waited {
        Waited::Exited(status) => status,
        unfinished => {
            dot_process::stop(&mut child).map_err(EngineError::Lost)?;
            return Err(match unfinished {
                Waited::Cancelled => EngineError::Cancelled,
                _ => EngineError::TimedOut(timeout),
            });
        }
    };
    Ok(Ended {
        status,
        output: heard(output)?,
        errors: heard(errors)?,
    })
}

pub(crate) fn listen(mut from: impl Read + Send + 'static) -> JoinHandle<io::Result<String>> {
    thread::spawn(move || {
        let mut kept = Vec::new();
        (&mut from).take(LONGEST_OUTPUT).read_to_end(&mut kept)?;
        io::copy(&mut from, &mut io::sink())?;
        Ok(String::from_utf8_lossy(&kept).into_owned())
    })
}

pub(crate) fn heard(reader: Option<JoinHandle<io::Result<String>>>) -> Result<String, EngineError> {
    let Some(reader) = reader else {
        return Ok(String::new());
    };
    reader
        .join()
        .unwrap_or_else(|_| Err(io::Error::other(READER_PANICKED)))
        .map_err(EngineError::Lost)
}

/// What a program that failed said: on its errors, or else where Docker's stand-in of a WSL without
/// Docker says it, on its output.
pub(crate) fn refusal(errors: &str, output: &str, status: ExitStatus) -> String {
    let said = [errors.trim(), output.trim()];
    let errors = said
        .into_iter()
        .find(|said| !said.is_empty())
        .unwrap_or_default();
    if errors.is_empty() {
        return status.to_string();
    }
    let skipped = errors.chars().count().saturating_sub(LONGEST_REFUSAL);
    errors.chars().skip(skipped).collect()
}
