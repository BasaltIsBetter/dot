use std::io::Read as _;
use std::time::Duration;

use dot_api::{AccessToken, ApiKey, Effort, Prompt};
use serde::Serialize;
use ureq::http::Response;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::{Agent, SendBody};

use crate::content::UserMessage;
use crate::stream::Replayed;
use crate::tool::{TOOLS, ToolResults};

pub(crate) const MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2026-10-21";
const MAX_TOKENS: u32 = 64_000;
const BETAS: &str = "server-side-fallback-2026-07-01,thinking-binding-controls-2026-08-01";
const ACCOUNT_BETAS: &str =
    "server-side-fallback-2026-07-01,thinking-binding-controls-2026-08-01,oauth-2025-04-20";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const SEND_TIMEOUT: Duration = Duration::from_secs(60);
const ANSWER_TIMEOUT: Duration = Duration::from_secs(120);
const SILENCE_TIMEOUT: Duration = Duration::from_secs(300);
const TOOLS_FIELD: &[u8] = br#","tools":"#;
const MESSAGES_FIELD: &[u8] = br#","messages":["#;
const MESSAGES_END: &[u8] = b"]}";
const SEPARATOR: u8 = b',';

#[derive(Clone, Copy, Debug)]
pub(crate) enum Authorization<'a> {
    ApiKey(&'a ApiKey),
    Bearer(&'a AccessToken),
}

#[derive(Serialize)]
struct Settings<'a> {
    model: &'a str,
    max_tokens: u32,
    stream: bool,
    system: &'a str,
    thinking: Thinking,
    output_config: OutputConfig,
    cache_control: CacheControl,
    fallbacks: &'static str,
}

#[derive(Serialize)]
struct Thinking {
    #[serde(rename = "type")]
    kind: &'static str,
    display: &'static str,
    block_binding: BlockBinding,
}

#[derive(Serialize)]
struct BlockBinding {
    prefix_mismatch_behavior: &'static str,
}

#[derive(Serialize)]
struct OutputConfig {
    effort: &'static str,
}

#[derive(Serialize)]
struct CacheControl {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Serialize)]
struct AssistantMessage<'a> {
    role: &'static str,
    content: &'a [Replayed<'a>],
}

pub(crate) fn agent() -> Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    Agent::config_builder()
        .tls_config(tls)
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_send_request(Some(SEND_TIMEOUT))
        .timeout_recv_response(Some(ANSWER_TIMEOUT))
        .timeout_per_read(Some(SILENCE_TIMEOUT))
        .build()
        .into()
}

pub(crate) fn post(
    agent: &Agent,
    url: &str,
    authorization: Authorization<'_>,
    head: &[u8],
    history: &[u8],
) -> Result<Response<ureq::Body>, ureq::Error> {
    let length = head.len() + history.len() + MESSAGES_END.len();
    let request = agent
        .post(url)
        .header("content-type", "application/json")
        .header("content-length", length)
        .header("anthropic-version", API_VERSION);
    let request = match authorization {
        Authorization::ApiKey(key) => request
            .header("x-api-key", key.expose())
            .header("anthropic-beta", BETAS),
        Authorization::Bearer(token) => request
            .header("authorization", format!("Bearer {}", token.expose()))
            .header("anthropic-beta", ACCOUNT_BETAS),
    };
    let mut body = head.chain(history).chain(MESSAGES_END);
    request.send(SendBody::from_reader(&mut body))
}

pub(crate) fn head(model: &str, effort: Effort, system: &str) -> serde_json::Result<Vec<u8>> {
    let settings = Settings {
        model,
        max_tokens: MAX_TOKENS,
        stream: true,
        system,
        thinking: Thinking {
            kind: "adaptive",
            display: "summarized",
            block_binding: BlockBinding {
                prefix_mismatch_behavior: "drop_block",
            },
        },
        output_config: OutputConfig {
            effort: effort.api_name(),
        },
        cache_control: CacheControl { kind: "ephemeral" },
        fallbacks: "default",
    };
    let mut head = serde_json::to_vec(&settings)?;
    head.pop();
    head.extend_from_slice(TOOLS_FIELD);
    head.extend_from_slice(TOOLS.as_bytes());
    head.extend_from_slice(MESSAGES_FIELD);
    Ok(head)
}

pub(crate) fn append_user(history: &mut Vec<u8>, prompt: &Prompt) -> serde_json::Result<()> {
    append(history, &UserMessage::of(prompt))
}

pub(crate) fn append_assistant(
    history: &mut Vec<u8>,
    content: &[Replayed<'_>],
) -> serde_json::Result<bool> {
    if content.is_empty() {
        return Ok(false);
    }
    append(
        history,
        &AssistantMessage {
            role: "assistant",
            content,
        },
    )?;
    Ok(true)
}

pub(crate) fn append_tool_results(
    history: &mut Vec<u8>,
    results: &[(String, dot_api::ToolOutput)],
) -> serde_json::Result<()> {
    let message = ToolResults::of(results);
    if message.is_empty() {
        return Ok(());
    }
    append(history, &message)
}

fn append(history: &mut Vec<u8>, message: &impl Serialize) -> serde_json::Result<()> {
    let length = history.len();
    if length > 0 {
        history.push(SEPARATOR);
    }
    serde_json::to_writer(&mut *history, message).inspect_err(|_| history.truncate(length))
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/messages/request.rs"]
mod tests;
