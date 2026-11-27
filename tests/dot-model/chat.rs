use std::io::Write;
use std::net::TcpListener;
use std::thread;

use dot_api::{AccessToken, ApiKey, Cost, Decision, Effort, Model, ReplyFailure};

use super::*;

const fn tuned(model: Model, effort: Effort) -> Tuning {
    Tuning {
        model,
        effort,
        summaries: Summaries::LeftToClaude,
        compaction: dot_api::OutputCompaction::FULL,
    }
}
use crate::messages::tests::{REPLY, read_request, serve, streamed};
use crate::tool::tests::{Answering, FakeToolbox, Untouched};

const UNAUTHORIZED: &str =
    "HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";

fn refusing_server(requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/messages", listener.local_addr().unwrap());
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            // The whole request is read: Linux resets a connection closed with some of it unread,
            // and the answer is lost.
            assert_ne!(read_request(&stream).head.len(), 0);
            stream.write_all(UNAUTHORIZED.as_bytes()).unwrap();
        }
    });
    url
}

fn chat_with(url: &str) -> ClaudeChat<FakeToolbox, Untouched> {
    ClaudeChat {
        toolbox: FakeToolbox::without_folder(),
        compactor: Untouched,
        messages: MessagesChat::at(url, Model::OPUS_5_5, Effort::High),
        claude_code: ClaudeCodeChat::new(Model::OPUS_5_5, Effort::High, None, None, None),
    }
}

fn text(prompt: &str) -> Prompt {
    Prompt {
        text: prompt.to_owned(),
        images: Vec::new(),
    }
}

fn failure_of(
    chat: &mut ClaudeChat<FakeToolbox, Untouched>,
    credential: &Credential,
) -> ReplyFailure {
    chat.reply(
        text("Salut"),
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
    let mut chat = chat_with(&refusing_server(2));
    let key = Credential::ApiKey(ApiKey::parse("sk-ant-api03-0123456789abcdef").unwrap());
    let token =
        Credential::AccessToken(AccessToken::parse("sk-ant-oat01-0123456789abcdef").unwrap());

    assert_eq!(failure_of(&mut chat, &key), ReplyFailure::Unauthorized);
    assert_eq!(failure_of(&mut chat, &token), ReplyFailure::Unauthorized);
}

#[test]
fn sends_a_claude_plan_through_claude_code() {
    let mut chat = chat_with("http://127.0.0.1:9/v1/messages");

    assert_eq!(
        failure_of(&mut chat, &Credential::ClaudeCode),
        ReplyFailure::ClaudeCodeFailed
    );
}

#[test]
fn creates_nothing_before_the_first_reply() {
    let workspace = std::env::temp_dir().join(format!("dot-chat-untouched-{}", std::process::id()));

    let _chat = ClaudeChat::new(
        tuned(Model::OPUS_5_5, Effort::High),
        Some(PathBuf::from("/usr/bin/true")),
        Some(workspace.clone()),
        None,
        FakeToolbox::without_folder(),
        Untouched,
    );

    assert!(!workspace.exists());
}

fn ask(
    chat: &mut ClaudeChat<FakeToolbox, Untouched>,
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
            prompt,
            credential,
            &Cancellation::new(),
            &Answering::with(Decision::Deny),
            &mut |event| match event {
                ReplyEvent::Thinking(piece) => thinking.push_str(&piece),
                ReplyEvent::Text(piece) => text.push_str(&piece),
                ReplyEvent::ToolStarted { .. }
                | ReplyEvent::ToolOutput { .. }
                | ReplyEvent::ToolChanged { .. }
                | ReplyEvent::ToolEnded { .. }
                | ReplyEvent::Agent { .. }
                | ReplyEvent::Compacted { .. } => {}
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
    let mut chat = ClaudeChat::new(
        tuned(Model::OPUS_5_5, Effort::High),
        None,
        None,
        None,
        FakeToolbox::without_folder(),
        Untouched,
    );

    let (first, _, first_text) = ask(
        &mut chat,
        &credential,
        "Pick a fruit and answer with its name only.",
    );
    let (second, _, second_text) = ask(
        &mut chat,
        &credential,
        "Which fruit did you pick? One word.",
    );

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
    let mut chat = ClaudeChat::new(
        tuned(Model::OPUS_5_5, Effort::High),
        claude,
        Some(workspace.clone()),
        None,
        FakeToolbox::without_folder(),
        Untouched,
    );

    let (first, _, first_text) = ask(
        &mut chat,
        &Credential::ClaudeCode,
        "Pick a fruit and answer with its name only.",
    );
    let (second, _, second_text) = ask(
        &mut chat,
        &Credential::ClaudeCode,
        "Which fruit did you pick? One word.",
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

fn sees_the_colour(
    chat: &mut ClaudeChat<FakeToolbox, Untouched>,
    credential: &Credential,
) -> String {
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
        prompt,
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
    let mut chat = ClaudeChat::new(
        tuned(Model::OPUS_5_5, Effort::Low),
        None,
        None,
        None,
        FakeToolbox::without_folder(),
        Untouched,
    );

    assert!(sees_the_colour(&mut chat, &Credential::AccessToken(token)).contains("rouge"));
}

#[test]
#[ignore = "runs the signed-in Claude Code, on the Claude plan"]
fn claude_reads_an_image_through_claude_code() {
    let workspace = std::env::temp_dir().join(format!("dot-claude-image-{}", std::process::id()));
    let claude = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/bin/claude"));
    let mut chat = ClaudeChat::new(
        tuned(Model::OPUS_5_5, Effort::Low),
        claude,
        Some(workspace.clone()),
        None,
        FakeToolbox::without_folder(),
        Untouched,
    );

    let colour = sees_the_colour(&mut chat, &Credential::ClaudeCode);

    std::fs::remove_dir_all(&workspace).unwrap();
    assert!(colour.contains("rouge"), "{colour}");
}

#[test]
fn reset_forgets_the_conversation_of_the_messages_api() {
    let server = serve(vec![streamed(REPLY), streamed(REPLY)]);
    let mut chat = chat_with(&server.url);
    let key = Credential::ApiKey(ApiKey::parse("sk-ant-api03-0123456789abcdef").unwrap());

    ask(&mut chat, &key, "Salut");
    chat.reset();
    ask(&mut chat, &key, "Nouveau");

    server.requests.recv().unwrap();
    let second = server.requests.recv().unwrap();
    assert_eq!(
        second.body["messages"],
        serde_json::json!([{"role": "user", "content": "Nouveau"}])
    );
}

#[test]
#[ignore = "runs the signed-in Claude Code, on the Claude plan, with the dot-shell built by `cargo build -p dot-shell`"]
fn claude_reads_the_shortened_output_and_the_true_status_of_a_command_of_claude_code() {
    let workspace = std::env::temp_dir().join(format!("dot-claude-shell-{}", std::process::id()));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let claude = home.map(|home| home.join(".local/bin/claude"));
    let shell = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/dot-shell");
    let shell = std::fs::canonicalize(shell).unwrap();
    let mut chat = ClaudeChat::new(
        tuned(Model::HAIKU_5_5, Effort::Low),
        claude,
        Some(workspace.clone()),
        Some(shell),
        FakeToolbox::without_folder(),
        Untouched,
    );
    let prompt = Prompt {
        text: "Run exactly this with your Bash tool, in one call: sh -c 'for i in 1 2 3 4 5 6 \
               7 8 9 10 11 12; do echo \"src/some/long/path/file_name.rs:$i:    let value = \
               compute(input);\"; done; exit 3'  Then quote exactly what the tool returned, and \
               nothing else."
            .to_owned(),
        images: Vec::new(),
    };
    let (mut text, mut ends) = (String::new(), Vec::new());

    chat.reply(
        prompt,
        &Credential::ClaudeCode,
        &Cancellation::new(),
        &Answering::with(Decision::Allow),
        &mut |event| match event {
            ReplyEvent::Text(piece) => text.push_str(&piece),
            ReplyEvent::ToolEnded { end, .. } => ends.push(end),
            ReplyEvent::Thinking(_)
            | ReplyEvent::ToolStarted { .. }
            | ReplyEvent::ToolOutput { .. }
            | ReplyEvent::ToolChanged { .. }
            | ReplyEvent::Agent { .. }
            | ReplyEvent::Compacted { .. } => {}
        },
    )
    .unwrap();

    std::fs::remove_dir_all(&workspace).unwrap();
    assert!(text.contains("Exit code 3"), "{text}");
    assert!(
        text.contains("12:    let value = compute(input);"),
        "{text}"
    );
    assert!(!text.contains("file_name.rs:12:"), "{text}");
    assert_eq!(ends, [dot_api::ToolEnd::Failed]);
}

const NO_TOOL: &str = "Reply with the single word OK and use no tool.";
const WORDS: &str = "/usr/share/dict/words";
const WORDS_KEPT: usize = 330_000;
const MOST_PROMPTS_BEFORE_A_SUMMARY: usize = 70;

fn ask_and_hear_summaries(
    chat: &mut ClaudeChat<FakeToolbox, Untouched>,
    prompt: String,
) -> (Reply, Vec<(u64, u64)>) {
    let mut summaries = Vec::new();
    let reply = chat
        .reply(
            Prompt {
                text: prompt,
                images: Vec::new(),
            },
            &Credential::ClaudeCode,
            &Cancellation::new(),
            &Answering::with(Decision::Deny),
            &mut |event| {
                if let ReplyEvent::Compacted { before, after } = event {
                    summaries.push((before, after));
                }
            },
        )
        .unwrap();
    (reply, summaries)
}

#[test]
#[ignore = "runs the signed-in Claude Code some forty times on Haiku, on the Claude plan"]
fn claude_code_summarizes_a_long_conversation_between_two_prompts_once_its_surplus_has_paid_for_it()
{
    let workspace = std::env::temp_dir().join(format!("dot-summaries-{}", std::process::id()));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let claude = home.map(|home| home.join(".local/bin/claude"));
    let tuning = Tuning {
        model: Model::HAIKU_5_5,
        effort: Effort::Low,
        summaries: Summaries::WhenTheyPay,
        compaction: dot_api::OutputCompaction::FULL,
    };
    let mut chat = ClaudeChat::new(
        tuning,
        claude,
        Some(workspace.clone()),
        None,
        FakeToolbox::without_folder(),
        Untouched,
    );
    let mut words = std::fs::read(WORDS).unwrap();
    words.truncate(WORDS_KEPT);
    let long = format!(
        "Here is a word list I may ask about later. {NO_TOOL}\n{}",
        String::from_utf8_lossy(&words)
    );

    let (first, _) = ask_and_hear_summaries(&mut chat, NO_TOOL.to_owned());
    let (grown, _) = ask_and_hear_summaries(&mut chat, long);
    let mut summarized = None;
    let mut heard_during_a_reply = Vec::new();
    for prompt in 0..MOST_PROMPTS_BEFORE_A_SUMMARY {
        chat.settle(&Cancellation::new());
        let (reply, summaries) = ask_and_hear_summaries(&mut chat, NO_TOOL.to_owned());
        heard_during_a_reply.extend(summaries);
        if reply.context_tokens < grown.context_tokens / 2 {
            summarized = Some((prompt, reply));
            break;
        }
    }

    std::fs::remove_dir_all(&workspace).unwrap();
    let (prompts, reply) = summarized.expect("no summary in seventy prompts");
    eprintln!(
        "base {:?}, grown to {}, summarized between two prompts after {prompts} more: then {}; cost of that reply {:?}",
        first.base_tokens, grown.context_tokens, reply.context_tokens, reply.cost
    );
    assert!(prompts >= 8, "{prompts}");
    assert_eq!(heard_during_a_reply, [], "the summary made a reply wait");
}

const ONE_REQUEST: &str = r#"{"type":"stream_event","event":{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":9000,"output_tokens":1}}}}"#;
const DONE: &str = r#"{"type":"result","is_error":false,"total_cost_usd":0}"#;

struct ArgumentsKept {
    folder: PathBuf,
    workspace: PathBuf,
    program: PathBuf,
}

impl ArgumentsKept {
    fn new(name: &str) -> Self {
        let folder = std::env::temp_dir().join(format!("dot-chat-{}-{name}", std::process::id()));
        let workspace = folder.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let program = folder.join("claude");
        let script = format!(
            "#!/bin/sh\nIFS= read -r prompt\nprintf '%s\\n' \"$@\" >> arguments.txt\ncat <<'END'\n{ONE_REQUEST}\n{DONE}\nEND\n"
        );
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(
            &program,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
        Self {
            folder,
            workspace,
            program,
        }
    }

    fn chat(&self, tuning: Tuning) -> ClaudeChat<FakeToolbox, Untouched> {
        ClaudeChat::new(
            tuning,
            Some(self.program.clone()),
            Some(self.workspace.clone()),
            None,
            FakeToolbox::without_folder(),
            Untouched,
        )
    }

    fn launches(&self) -> String {
        std::fs::read_to_string(self.workspace.join("arguments.txt")).unwrap()
    }
}

impl Drop for ArgumentsKept {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

#[test]
fn passes_the_choice_of_summaries_on_to_claude_code() {
    let kept = ArgumentsKept::new("summaries");
    let mut chat = kept.chat(Tuning {
        summaries: Summaries::WhenTheyPay,
        ..tuned(Model::OPUS_5_5, Effort::High)
    });

    for _ in 0..11 {
        ask(&mut chat, &Credential::ClaudeCode, "Salut");
    }

    assert_eq!(
        kept.launches().matches("--autocompact\n").count(),
        1,
        "{}",
        kept.launches()
    );
}

#[test]
fn leaves_summaries_to_claude_when_the_user_set_them_in_claude_code() {
    let kept = ArgumentsKept::new("summaries-allowed");
    let mut chat = kept
        .chat(Tuning {
            summaries: Summaries::WhenTheyPay,
            ..tuned(Model::OPUS_5_5, Effort::High)
        })
        .summaries_allowed(Summaries::LeftToClaude);

    for _ in 0..11 {
        ask(&mut chat, &Credential::ClaudeCode, "Salut");
    }

    assert!(
        !kept.launches().contains("--autocompact"),
        "{}",
        kept.launches()
    );
}

#[test]
fn answers_the_next_prompt_with_the_new_tuning() {
    let kept = ArgumentsKept::new("tuned");
    let mut chat = kept.chat(tuned(Model::OPUS_5_5, Effort::High));
    ask(&mut chat, &Credential::ClaudeCode, "Salut");

    chat.tune(tuned(Model::SONNET_5_5, Effort::Low));
    ask(&mut chat, &Credential::ClaudeCode, "Encore");

    let launches = kept.launches();
    let (first, second) = launches.split_at(launches.rfind("--effort").unwrap());
    assert!(
        first.contains("claude-opus-5-5\n") && first.contains("high\n"),
        "{first}"
    );
    assert!(
        second.contains("claude-sonnet-5-5\n") && second.contains("low\n"),
        "{second}"
    );
    assert!(!second.contains("claude-opus-5-5"), "{second}");
}
fn claude_code_chat(
    claude: &crate::claude_code::tests::FakeClaude,
) -> ClaudeChat<FakeToolbox, Untouched> {
    ClaudeChat::new(
        tuned(Model::OPUS_5_5, Effort::High),
        Some(claude.program()),
        Some(claude.workspace()),
        None,
        FakeToolbox::without_folder(),
        Untouched,
    )
}

#[test]
fn passes_the_choice_of_reader_on_to_claude_code() {
    let claude = crate::claude_code::tests::long_conversation("chat-reader");
    let mut with = claude_code_chat(&claude).reading(Reading::ByACheaperAgent);

    ask(&mut with, &Credential::ClaudeCode, "Salut");
    let with_reader = claude.read("arguments.txt");
    let mut without = claude_code_chat(&claude);
    ask(&mut without, &Credential::ClaudeCode, "Salut");
    let both = claude.read("arguments.txt");

    assert_eq!(
        with_reader.matches("--agents\n").count(),
        1,
        "{with_reader}"
    );
    assert_eq!(both.matches("--agents\n").count(), 1, "{both}");
}

#[test]
fn passes_the_login_path_on_to_claude_code() {
    let claude = crate::claude_code::tests::FakeClaude::telling_its_path("chat-login-path");
    let mut chat = claude_code_chat(&claude).login_path(claude.login_path());

    ask(&mut chat, &Credential::ClaudeCode, "Salut");

    assert_eq!(
        claude.read("path.txt"),
        format!("{}\n", crate::claude_code::tests::FAKE_LOGIN_PATH)
    );
}

#[test]
fn settles_the_conversation_claude_code_carries() {
    let claude = crate::claude_code::tests::long_conversation("chat-settle");
    let mut chat = claude_code_chat(&claude);
    chat.tune(Tuning {
        summaries: Summaries::WhenTheyPay,
        ..tuned(Model::OPUS_5_5, Effort::High)
    });
    for _ in 0..crate::claude_code::tests::REPLIES_OF_A_LONG_CONVERSATION {
        ask(&mut chat, &Credential::ClaudeCode, "Salut");
    }

    chat.settle(&Cancellation::new());

    let prompts = claude.read("prompts.txt");
    assert!(
        prompts
            .lines()
            .last()
            .unwrap()
            .contains(r#""content":"/compact""#),
        "{prompts}"
    );
}
#[test]
fn passes_the_choice_about_instructions_on_to_claude_code() {
    let claude = crate::claude_code::tests::long_conversation("chat-instructions");
    std::fs::create_dir_all(claude.workspace()).unwrap();
    std::fs::write(claude.workspace().join("CLAUDE.md"), "Rules.\n").unwrap();
    let mut handing = claude_code_chat(&claude).instructions(Instructions::HandedByDot);

    ask(&mut handing, &Credential::ClaudeCode, "Salut");
    let mut loading = claude_code_chat(&claude);
    ask(&mut loading, &Credential::ClaudeCode, "Salut");

    let arguments = claude.read("arguments.txt");
    assert_eq!(arguments.matches("--settings\n").count(), 1, "{arguments}");
}

const SECRET_RULE: &str = "The secret word of this project is PAMPLEMOUSSE. When someone asks for the secret word of this project, answer PAMPLEMOUSSE.";

#[test]
#[ignore = "runs the signed-in Claude Code twice on Haiku, on the Claude plan"]
fn claude_code_follows_the_instructions_dot_hands_over_and_keeps_them_cached() {
    let workspace = std::env::temp_dir().join(format!("dot-instructions-{}", std::process::id()));
    std::fs::create_dir_all(&workspace).unwrap();
    let filler = "This line only makes the instructions long enough to see them in the cache.\n"
        .repeat(3_000);
    std::fs::write(
        workspace.join("CLAUDE.md"),
        format!("{filler}\n{SECRET_RULE}\n"),
    )
    .unwrap();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let claude = home.map(|home| home.join(".local/bin/claude"));
    let mut chat = ClaudeChat::new(
        tuned(Model::HAIKU_5_5, Effort::Low),
        claude,
        Some(workspace.clone()),
        None,
        FakeToolbox::without_folder(),
        Untouched,
    )
    .instructions(Instructions::HandedByDot);

    let (first, _, word) = ask(
        &mut chat,
        &Credential::ClaudeCode,
        "What is the secret word of this project? One word, no tool.",
    );
    chat.reset();
    let (afresh, _, again) = ask(
        &mut chat,
        &Credential::ClaudeCode,
        "Tell me the secret word of this project, in one word, without any tool.",
    );

    std::fs::remove_dir_all(&workspace).unwrap();
    eprintln!("first conversation {first:?}, a new one {afresh:?}");
    assert!(word.contains("PAMPLEMOUSSE"), "{word}");
    assert!(again.contains("PAMPLEMOUSSE"), "{again}");
    assert!(
        afresh.cost.micro_dollars() * 4 < first.cost.micro_dollars() * 3,
        "{:?} {:?}",
        first.cost,
        afresh.cost
    );
}

struct Recording(dot_api::OutputCompaction);

impl Compactor for Recording {
    fn compact(&self, _: dot_api::Origin<'_>, _: &str) -> Option<String> {
        None
    }

    fn readable<'a>(&self, output: &'a str) -> std::borrow::Cow<'a, str> {
        std::borrow::Cow::Borrowed(output)
    }

    fn tune(&mut self, compaction: dot_api::OutputCompaction) {
        self.0 = compaction;
    }
}

#[test]
fn shortens_outputs_as_the_tuning_says_from_the_start_and_after_each_change() {
    let quiet = dot_api::OutputCompaction {
        cleaning: dot_api::Toggle::Off,
        contents_cut: dot_api::CutLevel::Never,
        ..dot_api::OutputCompaction::FULL
    };
    let mut chat = ClaudeChat::new(
        Tuning {
            compaction: quiet,
            ..tuned(Model::OPUS_5_5, Effort::High)
        },
        None,
        None,
        None,
        FakeToolbox::without_folder(),
        Recording(dot_api::OutputCompaction::FULL),
    );
    assert_eq!(chat.compactor.0, quiet);

    chat.tune(tuned(Model::OPUS_5_5, Effort::High));

    assert_eq!(chat.compactor.0, dot_api::OutputCompaction::FULL);
}

const KEPT_SESSION: &str = "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0";
const KEPT_MESSAGES: &str = r#"{"role":"user","content":"Salut"},{"role":"assistant","content":[{"type":"text","text":"Bonjour"}]}"#;

fn memory_of(chat: &ClaudeChat<FakeToolbox, Untouched>) -> String {
    let mut memory = Vec::new();
    chat.remember(&mut memory).unwrap();
    String::from_utf8(memory).unwrap()
}

fn kept_memory() -> Vec<u8> {
    format!(
        "{{\"format\":1,\"claude-code\":{{\"session\":\"{KEPT_SESSION}\",\"billed\":12}}}}\n{KEPT_MESSAGES}"
    )
    .into_bytes()
}

#[test]
fn remembers_nothing_of_a_conversation_not_started() {
    let memory = memory_of(&chat_with("http://127.0.0.1:9/v1/messages"));

    let (header, messages) = memory.split_once('\n').unwrap();
    let header: Value = serde_json::from_str(header).unwrap();
    assert_eq!(header["format"], 1);
    assert_eq!(header["claude-code"].get("session"), None);
    assert_eq!(messages, "");
}

#[test]
fn recalls_what_it_remembered() {
    let mut chat = chat_with("http://127.0.0.1:9/v1/messages");
    chat.recall(kept_memory());
    let remembered = memory_of(&chat);

    let mut again = chat_with("http://127.0.0.1:9/v1/messages");
    again.recall(remembered.clone().into_bytes());

    assert_eq!(memory_of(&again), remembered);
    assert!(
        remembered.ends_with(&format!("\n{KEPT_MESSAGES}")),
        "{remembered}"
    );
    assert!(remembered.contains(KEPT_SESSION), "{remembered}");
    assert!(remembered.contains(r#""billed":12"#), "{remembered}");
}

#[test]
fn goes_on_with_the_recalled_conversation_of_the_messages_api() {
    let server = serve(vec![streamed(REPLY)]);
    let mut chat = chat_with(&server.url);
    let key = Credential::ApiKey(ApiKey::parse("sk-ant-api03-0123456789abcdef").unwrap());

    chat.recall(kept_memory());
    ask(&mut chat, &key, "Encore");

    let request = server.requests.recv().unwrap();
    assert_eq!(
        request.body["messages"],
        serde_json::json!([
            {"role": "user", "content": "Salut"},
            {"role": "assistant", "content": [{"type": "text", "text": "Bonjour"}]},
            {"role": "user", "content": "Encore"}
        ])
    );
}

#[test]
fn a_memory_it_cannot_read_leaves_the_conversation_empty() {
    let fresh = memory_of(&chat_with("http://127.0.0.1:9/v1/messages"));
    let mut wrong_format = kept_memory();
    wrong_format[10] = b'2';

    for memory in [
        b"no header".to_vec(),
        b"{not json}\n".to_vec(),
        wrong_format,
        Vec::new(),
    ] {
        let mut chat = chat_with("http://127.0.0.1:9/v1/messages");
        chat.recall(kept_memory());

        chat.recall(memory.clone());

        assert_eq!(
            memory_of(&chat),
            fresh,
            "{}",
            String::from_utf8_lossy(&memory)
        );
    }
}
