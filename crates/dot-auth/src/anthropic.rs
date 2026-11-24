use std::time::Duration;

use dot_api::{ApiKey, Cancellation};
use ureq::Agent;
use ureq::config::Config;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::connector::{KeyVerifier, Verdict};

const MODELS_URL: &str = "https://api.anthropic.com/v1/models";
const API_VERSION: &str = "2026-10-21";
const KEY_HEADER: &str = "x-api-key";
const VERSION_HEADER: &str = "anthropic-version";
const TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct HttpKeyVerifier {
    config: Config,
    url: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Anthropic answered with HTTP status {0}")]
struct UnexpectedStatus(u16);

impl HttpKeyVerifier {
    pub(crate) fn new() -> Self {
        Self::at(MODELS_URL)
    }

    fn at(url: impl Into<String>) -> Self {
        let tls = TlsConfig::builder()
            .provider(TlsProvider::NativeTls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let config = Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .max_redirects(0)
            .build();

        Self {
            config,
            url: url.into(),
        }
    }
}

impl KeyVerifier for HttpKeyVerifier {
    fn verify(&self, key: &ApiKey, cancellation: &Cancellation) -> Verdict {
        let agent = Agent::new_with_config(self.config.clone());
        let interrupted = agent.clone();
        let _cut_when_cancelled = cancellation.on_cancel(move || interrupted.shutdown());
        let response = agent
            .get(&self.url)
            .header(KEY_HEADER, key.expose())
            .header(VERSION_HEADER, API_VERSION)
            .call();

        match response {
            Ok(response) => verdict(response.status().as_u16()),
            Err(error) => Verdict::Unreachable(Box::new(error)),
        }
    }
}

fn verdict(status: u16) -> Verdict {
    match status {
        200..=299 => Verdict::Accepted,
        401 | 403 => Verdict::Rejected,
        _ => Verdict::Unreachable(Box::new(UnexpectedStatus(status))),
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-auth/anthropic.rs"]
mod tests;
