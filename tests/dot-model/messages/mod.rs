use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use dot_api::{AccessToken, ApiKey, Cost, Decision, Limits};
use serde_json::{Value, json};

use super::*;
use crate::messages::response::tests::REPLY;
use crate::tool::tests::{Answering, FakeToolbox};

const KEY: &str = "sk-ant-api03-0123456789abcdef";
const TOKEN: &str = "sk-ant-oat01-0123456789abcdef";
const STREAM_HEAD: &str =
    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
const OVERLOADED_BODY: &str =
    r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
const UNAUTHORIZED: &str =
    "HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";

struct Request {
    head: Vec<String>,
    body: Value,
}

struct Server {
    url: String,
    requests: mpsc::Receiver<Request>,
}

fn serve(answers: Vec<String>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/messages", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    thread::spawn(move || {
        for answer in answers {
            let (stream, _) = listener.accept().unwrap();
            let request = read_request(&stream);
            sender.send(request).unwrap();
            answer_with(stream, &answer);
        }
    });
    Server { url, requests }
}

fn read_request(stream: &TcpStream) -> Request {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let head: Vec<String> = (&mut reader)
        .lines()
        .map(Result::unwrap)
        .take_while(|line| !line.is_empty())
        .collect();
    let length: usize = head
        .iter()
        .find_map(|line| line.strip_prefix("content-length: "))
        .unwrap()
        .parse()
        .unwrap();
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    Request {
        head,
        body: serde_json::from_slice(&body).unwrap(),
    }
}

fn answer_with(mut stream: TcpStream, answer: &str) {
    if let Some(pause) = answer.strip_prefix("hold:") {
        stream.write_all(pause.as_bytes()).unwrap();
        thread::sleep(Duration::from_secs(10));
        return;
    }
    stream.write_all(answer.as_bytes()).unwrap();
}

fn overloaded() -> String {
    format!(
        "HTTP/1.1 529 Overloaded\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{OVERLOADED_BODY}",
        OVERLOADED_BODY.len()
    )
}

fn streamed(events: &str) -> String {
    format!("{STREAM_HEAD}{events}")
}

fn key() -> ApiKey {
    ApiKey::parse(KEY).unwrap()
}

fn text(prompt: &str) -> Prompt {
    Prompt {
        text: prompt.to_owned(),
        images: Vec::new(),
    }
}

fn reply_to(chat: &MessagesChat, prompt: &str) -> (Result<Reply, ReplyError>, Vec<ReplyEvent>) {
    let mut heard = Vec::new();
    let key = key();
    let result = chat.reply(
        &text(prompt),
        Authorization::ApiKey(&key),
        &Hands {
            toolbox: &FakeToolbox::without_folder(),
            permissions: &Answering::with(Decision::Deny),
            cancellation: &Cancellation::new(),
        },
        &mut |event| heard.push(event),
    );
    (result, heard)
}

#[test]
fn streams_the_reply_and_reports_what_it_used() {
    let server = serve(vec![streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (result, heard) = reply_to(&chat, "Salut");

    let reply = result.unwrap();
    assert_eq!(heard, [ReplyEvent::Text("Bonjour".into())]);
    assert_eq!(reply.context_tokens, 15);
    assert_eq!(reply.cost, Cost::from_micro_dollars(140));
    assert_eq!(reply.limits, Limits::default());
}

#[test]
fn sends_the_key_the_version_and_the_betas_in_headers() {
    let server = serve(vec![streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    reply_to(&chat, "Salut").0.unwrap();

    let request = server.requests.recv().unwrap();
    assert_eq!(request.head[0], "POST /v1/messages HTTP/1.1");
    assert!(request.head.contains(&format!("x-api-key: {KEY}")));
    assert!(
        request
            .head
            .contains(&"anthropic-version: 2026-10-21".to_owned())
    );
    assert!(
        request.head.contains(
            &"anthropic-beta: server-side-fallback-2026-07-01,thinking-binding-controls-2026-08-01"
                .to_owned()
        )
    );
    assert!(
        !request
            .head
            .iter()
            .any(|line| line.starts_with("authorization"))
    );
}

#[test]
fn sends_an_account_token_as_bearer_with_the_oauth_beta() {
    let server = serve(vec![streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let token = AccessToken::parse(TOKEN).unwrap();

    chat.reply(
        &text("Salut"),
        Authorization::Bearer(&token),
        &Hands {
            toolbox: &FakeToolbox::without_folder(),
            permissions: &Answering::with(Decision::Deny),
            cancellation: &Cancellation::new(),
        },
        &mut |_| {},
    )
    .unwrap();

    let request = server.requests.recv().unwrap();
    assert!(
        request
            .head
            .contains(&format!("authorization: Bearer {TOKEN}"))
    );
    assert!(
        request
            .head
            .iter()
            .any(|line| line.starts_with("anthropic-beta") && line.ends_with("oauth-2025-04-20"))
    );
    assert!(
        !request
            .head
            .iter()
            .any(|line| line.starts_with("x-api-key"))
    );
}

#[test]
fn carries_the_conversation_from_one_prompt_to_the_next() {
    let server = serve(vec![streamed(REPLY), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    reply_to(&chat, "Salut").0.unwrap();
    reply_to(&chat, "Encore").0.unwrap();

    server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    assert_eq!(
        second.body["messages"],
        json!([
            {"role": "user", "content": "Salut"},
            {"role": "assistant", "content": [{"type": "text", "text": "Bonjour"}]},
            {"role": "user", "content": "Encore"},
        ])
    );
}

#[test]
fn retries_an_overloaded_claude_then_streams() {
    let server = serve(vec![overloaded(), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (result, heard) = reply_to(&chat, "Salut");

    assert!(result.is_ok());
    assert_eq!(heard.len(), 1);
    assert_eq!(server.requests.iter().count(), 2);
}

#[test]
fn gives_up_after_its_retries() {
    let server = serve(vec![overloaded(); 3]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (result, _) = reply_to(&chat, "Salut");

    let error = result.unwrap_err();
    assert_eq!(error.failure(), ReplyFailure::Overloaded);
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(server.requests.iter().count(), 3);
}

#[test]
fn never_retries_a_refused_credential() {
    let server = serve(vec![UNAUTHORIZED.to_owned()]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (result, _) = reply_to(&chat, "Salut");

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Unauthorized);
    assert_eq!(server.requests.iter().count(), 1);
}

#[test]
fn forgets_a_declined_reply_but_keeps_the_prompt() {
    let declined = REPLY.replace("end_turn", "refusal");
    let server = serve(vec![streamed(&declined), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (result, _) = reply_to(&chat, "Salut");
    reply_to(&chat, "Autre chose").0.unwrap();

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Declined);
    server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    assert_eq!(
        second.body["messages"],
        json!([
            {"role": "user", "content": "Salut"},
            {"role": "user", "content": "Autre chose"},
        ])
    );
}

#[test]
fn stops_at_once_when_cancelled_mid_reply() {
    let first_piece = &REPLY[..REPLY.find("event: content_block_stop").unwrap()];
    let server = serve(vec![format!("hold:{STREAM_HEAD}{first_piece}")]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let cancellation = Cancellation::new();
    let (heard_sender, heard) = mpsc::channel();

    let (result, stopped_after) = thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let key = key();
            chat.reply(
                &text("Salut"),
                Authorization::ApiKey(&key),
                &Hands {
                    toolbox: &FakeToolbox::without_folder(),
                    permissions: &Answering::with(Decision::Deny),
                    cancellation: &cancellation,
                },
                &mut |event| {
                    heard_sender.send(event).unwrap();
                },
            )
        });
        heard.recv_timeout(Duration::from_secs(5)).unwrap();
        let cancelled_at = Instant::now();
        cancellation.cancel();
        let result = worker.join().unwrap();
        (result, cancelled_at.elapsed())
    });

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Stopped);
    assert!(stopped_after < Duration::from_secs(1));
}

#[test]
fn keeps_what_was_written_before_a_stop() {
    let first_piece = &REPLY[..REPLY.find("event: content_block_stop").unwrap()];
    let server = serve(vec![format!("{STREAM_HEAD}{first_piece}"), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (cut, _) = reply_to(&chat, "Salut");
    reply_to(&chat, "Suite").0.unwrap();

    assert_eq!(cut.unwrap_err().failure(), ReplyFailure::Unreachable);
    server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    assert_eq!(
        second.body["messages"][1]["content"],
        json!([{"type": "text", "text": "Bonjour"}])
    );
}

#[test]
fn reports_an_anthropic_it_cannot_reach() {
    let chat = MessagesChat::at(
        "http://127.0.0.1:9/v1/messages",
        Model::OPUS_5_5,
        Effort::High,
    );

    let (result, heard) = reply_to(&chat, "Salut");

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Unreachable);
    assert_eq!(heard, []);
}

#[test]
fn sends_the_images_of_a_prompt_before_its_text() {
    let server = serve(vec![streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let png = b"\x89PNG\r\n\x1a\n pixels".to_vec();
    let prompt = Prompt {
        text: "Et ça ?".to_owned(),
        images: vec![dot_api::Image::from_bytes(png).unwrap()],
    };
    let key = key();

    chat.reply(
        &prompt,
        Authorization::ApiKey(&key),
        &Hands {
            toolbox: &FakeToolbox::without_folder(),
            permissions: &Answering::with(Decision::Deny),
            cancellation: &Cancellation::new(),
        },
        &mut |_| {},
    )
    .unwrap();

    let request = server.requests.recv().unwrap();
    let content = &request.body["messages"][0]["content"];
    assert_eq!(content[0]["type"], "image");
    assert_eq!(content[0]["source"]["media_type"], "image/png");
    assert_eq!(content[1], json!({"type": "text", "text": "Et ça ?"}));
}

fn sse(events: &[&str]) -> String {
    events
        .iter()
        .flat_map(|event| ["data: ", event, "\n\n"])
        .collect()
}

fn wants_a_command(stop_reason: &str) -> String {
    streamed(&sse(&[
        r#"{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":1}}}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"bash","input":{}}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\":"}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"cargo test\"}"}}"#,
        r#"{"type":"content_block_stop","index":0}"#,
        &format!(
            r#"{{"type":"message_delta","delta":{{"stop_reason":"{stop_reason}"}},"usage":{{"output_tokens":5}}}}"#
        ),
        r#"{"type":"message_stop"}"#,
    ]))
}

fn reply_with_tools(
    chat: &MessagesChat,
    toolbox: &FakeToolbox,
    permissions: &Answering,
) -> (Result<Reply, ReplyError>, Vec<ReplyEvent>) {
    let mut heard = Vec::new();
    let key = key();
    let hands = Hands {
        toolbox,
        permissions,
        cancellation: &Cancellation::new(),
    };
    let result = chat.reply(
        &text("Lance les tests"),
        Authorization::ApiKey(&key),
        &hands,
        &mut |event| heard.push(event),
    );
    (result, heard)
}

#[test]
fn runs_the_tools_claude_asks_for_then_carries_on_until_it_finishes() {
    let server = serve(vec![wants_a_command("tool_use"), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let toolbox = FakeToolbox::in_folder("/project");
    let permissions = Answering::with(Decision::Allow);

    let (result, heard) = reply_with_tools(&chat, &toolbox, &permissions);

    let reply = result.unwrap();
    assert_eq!(
        toolbox.calls(),
        [(
            "toolu_1".to_owned(),
            dot_api::ToolCall::Run {
                command: "cargo test".to_owned()
            }
        )]
    );
    assert_eq!(permissions.asked().len(), 1);
    assert_eq!(
        heard,
        [
            ReplyEvent::ToolStarted {
                id: "toolu_1".to_owned(),
                tool: dot_api::ToolUse {
                    kind: dot_api::ToolKind::Run,
                    subject: "cargo test".to_owned()
                },
            },
            ReplyEvent::ToolEnded {
                id: "toolu_1".to_owned(),
                end: dot_api::ToolEnd::Done(dot_api::Detail::Exit(0)),
            },
            ReplyEvent::Text("Bonjour".into()),
        ]
    );
    assert_eq!(reply.cost, Cost::from_micro_dollars(280));
    assert_eq!(reply.context_tokens, 15);

    let first = server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    assert!(
        first.body["system"]
            .as_str()
            .unwrap()
            .contains("The user's project is the folder /project.")
    );
    assert_eq!(first.body["tools"], second.body["tools"]);
    assert_eq!(
        second.body["messages"],
        json!([
            {"role": "user", "content": "Lance les tests"},
            {"role": "assistant", "content": [
                {"type": "tool_use", "id": "toolu_1", "name": "bash",
                 "input": {"command": "cargo test"}},
            ]},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "ran cargo test"},
            ]},
        ])
    );
}

#[test]
fn tells_claude_when_the_user_refuses_a_tool() {
    let server = serve(vec![wants_a_command("tool_use"), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);

    let (result, heard) = reply_with_tools(
        &chat,
        &FakeToolbox::in_folder("/project"),
        &Answering::with(Decision::Deny),
    );

    result.unwrap();
    assert!(heard.contains(&ReplyEvent::ToolEnded {
        id: "toolu_1".to_owned(),
        end: dot_api::ToolEnd::Refused,
    }));
    server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    assert_eq!(
        second.body["messages"][2]["content"][0],
        json!({"type": "tool_result", "tool_use_id": "toolu_1",
               "content": dot_api::USER_REFUSED_THE_TOOL, "is_error": true})
    );
}

#[test]
fn answers_every_tool_even_when_the_reply_ends_before_running_it() {
    let server = serve(vec![wants_a_command("max_tokens"), streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let toolbox = FakeToolbox::in_folder("/project");

    let (result, heard) = reply_with_tools(&chat, &toolbox, &Answering::with(Decision::Allow));
    reply_to(&chat, "Encore").0.unwrap();

    result.unwrap();
    assert_eq!(toolbox.calls(), []);
    assert_eq!(
        heard.last(),
        Some(&ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: dot_api::ToolEnd::Stopped,
        })
    );
    server.requests.recv().unwrap();
    let next = server.requests.recv().unwrap();
    assert_eq!(
        next.body["messages"][2]["content"][0]["content"],
        "The reply ended before this tool ran."
    );
    assert_eq!(
        next.body["messages"][3],
        json!({"role": "user", "content": "Encore"})
    );
}

#[test]
fn resumes_a_paused_turn_without_adding_anything() {
    let paused = streamed(&sse(&[
        r#"{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":1}}}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"server_tool_use","id":"srvtoolu_1","name":"web_search","input":{}}}"#,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"query\":\"ureq\"}"}}"#,
        r#"{"type":"content_block_stop","index":0}"#,
        r#"{"type":"content_block_start","index":1,"content_block":{"type":"web_search_tool_result","tool_use_id":"srvtoolu_1","content":[{"type":"web_search_result","url":"https://docs.rs/ureq"}]}}"#,
        r#"{"type":"content_block_stop","index":1}"#,
        r#"{"type":"message_delta","delta":{"stop_reason":"pause_turn"},"usage":{"output_tokens":5}}"#,
        r#"{"type":"message_stop"}"#,
    ]));
    let server = serve(vec![paused, streamed(REPLY)]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let toolbox = FakeToolbox::without_folder();

    let (result, heard) = reply_with_tools(&chat, &toolbox, &Answering::with(Decision::Allow));

    result.unwrap();
    assert_eq!(toolbox.calls(), []);
    assert_eq!(
        heard[..2],
        [
            ReplyEvent::ToolStarted {
                id: "srvtoolu_1".to_owned(),
                tool: dot_api::ToolUse {
                    kind: dot_api::ToolKind::SearchWeb,
                    subject: "ureq".to_owned()
                },
            },
            ReplyEvent::ToolEnded {
                id: "srvtoolu_1".to_owned(),
                end: dot_api::ToolEnd::Done(dot_api::Detail::Matches(1)),
            },
        ]
    );
    server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    let messages = second.body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[1]["content"],
        json!([
            {"type": "server_tool_use", "id": "srvtoolu_1", "name": "web_search",
             "input": {"query": "ureq"}},
            {"type": "web_search_tool_result", "tool_use_id": "srvtoolu_1",
             "content": [{"type": "web_search_result", "url": "https://docs.rs/ureq"}]},
        ])
    );
}

#[test]
fn stops_without_running_tools_once_cancelled() {
    let server = serve(vec![wants_a_command("tool_use")]);
    let chat = MessagesChat::at(&server.url, Model::OPUS_5_5, Effort::High);
    let toolbox = FakeToolbox::in_folder("/project");
    let cancellation = Cancellation::new();
    let key = key();
    let hands = Hands {
        toolbox: &toolbox,
        permissions: &Answering::with(Decision::Allow),
        cancellation: &cancellation,
    };

    let result = chat.reply(
        &text("Lance"),
        Authorization::ApiKey(&key),
        &hands,
        &mut |event| {
            if matches!(event, ReplyEvent::ToolStarted { .. }) {
                cancellation.cancel();
            }
        },
    );

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Stopped);
    assert_eq!(toolbox.calls(), []);
}
