use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

use dot_api::{AccessToken, ApiKey, Cost, Decision, ReplyFailure};

use super::*;
use crate::tool::tests::{Answering, FakeToolbox};

const UNAUTHORIZED: &str =
    "HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";

fn refusing_server(requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/messages", listener.local_addr().unwrap());
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let head: Vec<String> = BufReader::new(stream.try_clone().unwrap())
                .lines()
                .map(Result::unwrap)
                .take_while(|line| !line.is_empty())
                .collect();
            assert_ne!(head.len(), 0);
            stream.write_all(UNAUTHORIZED.as_bytes()).unwrap();
        }
    });
    url
}

fn chat_with(url: &str) -> ClaudeChat<FakeToolbox> {
    ClaudeChat {
        toolbox: FakeToolbox::without_folder(),
        messages: MessagesChat::at(url, Model::OPUS_5_5, Effort::High),
        claude_code: ClaudeCodeChat::new(Model::OPUS_5_5, Effort::High, None, None),
    }
}

fn text(prompt: &str) -> Prompt {
    Prompt {
        text: prompt.to_owned(),
        images: Vec::new(),
    }
}

fn failure_of(chat: &ClaudeChat<FakeToolbox>, credential: &Credential) -> ReplyFailure {
    chat.reply(
        &text("Salut"),
        credential,
        &Cancellation::new(),
        &Answering::with(Decision::Deny),
        &mut |_| {},
    )
    .unwrap_err()
    .failure()
}

#[test]
fn sends_an_api_key_or_an_account_to_the_messages_api() {
    let chat = chat_with(&refusing_server(2));
    let key = Credential::ApiKey(ApiKey::parse("sk-ant-api03-0123456789abcdef").unwrap());
    let token =
        Credential::AccessToken(AccessToken::parse("sk-ant-oat01-0123456789abcdef").unwrap());

    assert_eq!(failure_of(&chat, &key), ReplyFailure::Unauthorized);
    assert_eq!(failure_of(&chat, &token), ReplyFailure::Unauthorized);
}

#[test]
fn sends_a_claude_plan_through_claude_code() {
    let chat = chat_with("http://127.0.0.1:9/v1/messages");

    assert_eq!(
        failure_of(&chat, &Credential::ClaudeCode),
        ReplyFailure::ClaudeCodeFailed
    );
}

#[test]
fn creates_nothing_before_the_first_reply() {
    let workspace = std::env::temp_dir().join(format!("dot-chat-untouched-{}", std::process::id()));

    let _chat = ClaudeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        Some(PathBuf::from("/usr/bin/true")),
        Some(workspace.clone()),
        FakeToolbox::without_folder(),
    );

    assert!(!workspace.exists());
}

fn ask(
    chat: &ClaudeChat<FakeToolbox>,
    credential: &Credential,
    prompt: &str,
) -> (Reply, String, String) {
    let (mut thinking, mut text) = (String::new(), String::new());
    let prompt = Prompt {
        text: prompt.to_owned(),
        images: Vec::new(),
    };
    let reply = chat
        .reply(
            &prompt,
            credential,
            &Cancellation::new(),
            &Answering::with(Decision::Deny),
            &mut |event| match event {
                ReplyEvent::Thinking(piece) => thinking.push_str(&piece),
                ReplyEvent::Text(piece) => text.push_str(&piece),
                ReplyEvent::ToolStarted { .. } | ReplyEvent::ToolEnded { .. } => {}
            },
        )
        .unwrap();
    (reply, thinking, text)
}

#[test]
#[ignore = "calls api.anthropic.com with the token of the signed-in ant account, billed per use"]
fn claude_answers_twice_through_the_messages_api() {
    let printed = std::process::Command::new("ant")
        .args(["auth", "print-credentials", "--access-token"])
        .output()
        .unwrap();
    let token = AccessToken::parse(&String::from_utf8(printed.stdout).unwrap()).unwrap();
    let credential = Credential::AccessToken(token);
    let chat = ClaudeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        None,
        None,
        FakeToolbox::without_folder(),
    );

    let (first, _, first_text) = ask(
        &chat,
        &credential,
        "Choisis un fruit et réponds juste son nom.",
    );
    let (second, _, second_text) = ask(&chat, &credential, "Quel fruit as-tu choisi ? Un mot.");

    assert_ne!(first_text.trim(), "");
    assert!(
        second_text
            .to_lowercase()
            .contains(first_text.trim().to_lowercase().trim_end_matches('.'))
    );
    assert!(first.cost > Cost::default());
    assert!(second.context_tokens > first.context_tokens);
}

#[test]
#[ignore = "runs the signed-in Claude Code, on the Claude plan"]
fn claude_answers_twice_through_claude_code() {
    let workspace = std::env::temp_dir().join(format!("dot-claude-code-{}", std::process::id()));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let claude = home.map(|home| home.join(".local/bin/claude"));
    let chat = ClaudeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        claude,
        Some(workspace.clone()),
        FakeToolbox::without_folder(),
    );

    let (first, _, first_text) = ask(
        &chat,
        &Credential::ClaudeCode,
        "Choisis un fruit et réponds juste son nom.",
    );
    let (second, _, second_text) = ask(
        &chat,
        &Credential::ClaudeCode,
        "Quel fruit as-tu choisi ? Un mot.",
    );

    std::fs::remove_dir_all(&workspace).unwrap();
    assert_ne!(first_text.trim(), "");
    assert!(
        second_text
            .to_lowercase()
            .contains(first_text.trim().to_lowercase().trim_end_matches('.'))
    );
    assert!(second.context_tokens > first.context_tokens);
    assert!(second.limits.seven_days.is_some());
}

const RED_SQUARE: &str = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEklEQVR4nGP4z8CAFWEXHbQSACj/P8Fu7N9hAAAAAElFTkSuQmCC";

fn sees_the_colour(chat: &ClaudeChat<FakeToolbox>, credential: &Credential) -> String {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(RED_SQUARE)
        .unwrap();
    let prompt = Prompt {
        text: "De quelle couleur est cette image ? Un mot.".to_owned(),
        images: vec![dot_api::Image::from_bytes(bytes).unwrap()],
    };
    let mut text = String::new();
    chat.reply(
        &prompt,
        credential,
        &Cancellation::new(),
        &Answering::with(Decision::Deny),
        &mut |event| {
            if let ReplyEvent::Text(piece) = event {
                text.push_str(&piece);
            }
        },
    )
    .unwrap();
    text.to_lowercase()
}

#[test]
#[ignore = "calls api.anthropic.com with the token of the signed-in ant account, billed per use"]
fn claude_reads_an_image_through_the_messages_api() {
    let printed = std::process::Command::new("ant")
        .args(["auth", "print-credentials", "--access-token"])
        .output()
        .unwrap();
    let token = AccessToken::parse(&String::from_utf8(printed.stdout).unwrap()).unwrap();
    let chat = ClaudeChat::new(
        Model::OPUS_5_5,
        Effort::Low,
        None,
        None,
        FakeToolbox::without_folder(),
    );

    assert!(sees_the_colour(&chat, &Credential::AccessToken(token)).contains("rouge"));
}

#[test]
#[ignore = "runs the signed-in Claude Code, on the Claude plan"]
fn claude_reads_an_image_through_claude_code() {
    let workspace = std::env::temp_dir().join(format!("dot-claude-image-{}", std::process::id()));
    let claude = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/bin/claude"));
    let chat = ClaudeChat::new(
        Model::OPUS_5_5,
        Effort::Low,
        claude,
        Some(workspace.clone()),
        FakeToolbox::without_folder(),
    );

    let colour = sees_the_colour(&chat, &Credential::ClaudeCode);

    std::fs::remove_dir_all(&workspace).unwrap();
    assert!(colour.contains("rouge"), "{colour}");
}
