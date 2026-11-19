use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use dot_api::{Cancellation, Cost, Decision, Detail, ToolEnd, ToolKind, ToolUse};

use super::*;
use crate::tool::tests::{Answering, FakeToolbox};

const SESSION: &str = "3898f4c9-b1b3-480e-b180-b139f7eb2054";
const INIT: &str =
    r#"{"type":"system","subtype":"init","session_id":"3898f4c9-b1b3-480e-b180-b139f7eb2054"}"#;
const START: &str = r#"{"type":"stream_event","event":{"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":2,"cache_read_input_tokens":649,"cache_creation_input_tokens":1680,"output_tokens":1}}}}"#;
const THINKING: &str = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}}"#;
const TEXT: &str = r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}}"#;
const OK: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"ok"}}}"#;
const ASSISTANT: &str =
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"ok"}]}}"#;
const END: &str = r#"{"type":"stream_event","event":{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":4}}}"#;
const LIMITS: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":{"seven_day":{"utilization":0.5}}}}"#;
const RESULT: &str =
    r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.0136578}"#;

struct FakeClaude {
    folder: PathBuf,
}

impl FakeClaude {
    fn printing(name: &str, lines: &[&str], then: &str) -> Self {
        let folder = std::env::temp_dir().join(format!("dot-claude-{}-{name}", std::process::id()));
        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        fs::create_dir_all(folder.join("bin")).unwrap();
        let printed: String = lines.iter().flat_map(|line| [*line, "\n"]).collect();
        let script = format!(
            "#!/bin/sh\nIFS= read -r prompt\nprintf '%s\\n' \"$prompt\" > prompt.txt\nprintf '%s\\n' \"$@\" >> arguments.txt\ncat <<'EOF'\n{printed}EOF\n{then}\n"
        );
        let program = folder.join("bin/claude");
        fs::write(&program, script).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        Self { folder }
    }

    fn chat(&self) -> ClaudeCodeChat {
        ClaudeCodeChat::new(
            Model::OPUS_5_5,
            Effort::High,
            Some(self.folder.join("bin/claude")),
            Some(self.workspace()),
        )
    }

    fn workspace(&self) -> PathBuf {
        self.folder.join("Application Support/Dot")
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.workspace().join(name)).unwrap()
    }
}

impl Drop for FakeClaude {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.folder) {
            eprintln!("could not remove {}: {error}", self.folder.display());
        }
    }
}

fn text(prompt: &str) -> Prompt {
    Prompt {
        text: prompt.to_owned(),
        images: Vec::new(),
    }
}

fn reply_to(
    chat: &mut ClaudeCodeChat,
    prompt: &str,
) -> (Result<Reply, ReplyError>, Vec<ReplyEvent>) {
    reply_with(chat, prompt, &Answering::with(Decision::Deny))
}

fn reply_with(
    chat: &mut ClaudeCodeChat,
    prompt: &str,
    permissions: &Answering,
) -> (Result<Reply, ReplyError>, Vec<ReplyEvent>) {
    let mut heard = Vec::new();
    let hands = Hands {
        toolbox: &FakeToolbox::without_folder(),
        permissions,
        cancellation: &Cancellation::new(),
    };
    let result = chat.reply(text(prompt), &hands, &mut |event| {
        heard.push(event);
    });
    (result, heard)
}

fn whole_reply() -> Vec<&'static str> {
    vec![
        INIT, START, THINKING, TEXT, OK, ASSISTANT, END, LIMITS, RESULT,
    ]
}

#[test]
fn streams_the_text_and_reports_cost_context_and_limits() {
    let claude = FakeClaude::printing("whole", &whole_reply(), "");

    let (result, heard) = reply_to(&mut claude.chat(), "Salut");

    let reply = result.unwrap();
    assert_eq!(heard, [ReplyEvent::Text("ok".into())]);
    assert_eq!(reply.cost, Cost::from_micro_dollars(13_658));
    assert_eq!(reply.context_tokens, 2 + 649 + 1_680 + 4);
    assert_eq!(reply.limits.seven_days, Some(50));
}

#[test]
fn hands_the_prompt_over_on_its_input_in_dot_s_folder() {
    let claude = FakeClaude::printing("prompt", &whole_reply(), "");

    reply_to(&mut claude.chat(), "--help\nmais pas comme option")
        .0
        .unwrap();

    let handed: serde_json::Value = serde_json::from_str(&claude.read("prompt.txt")).unwrap();
    assert_eq!(
        handed,
        serde_json::json!({"type": "user", "message": {"role": "user", "content": "--help\nmais pas comme option"}})
    );
    assert!(
        claude
            .read("arguments.txt")
            .contains("--permission-prompt-tool\nstdio\n")
    );
}

#[test]
fn resumes_the_conversation_on_the_next_prompt() {
    let claude = FakeClaude::printing("resume", &whole_reply(), "");
    let mut chat = claude.chat();

    reply_to(&mut chat, "Salut").0.unwrap();
    reply_to(&mut chat, "Encore").0.unwrap();

    let launches = claude.read("arguments.txt");
    assert_eq!(launches.matches("--resume\n").count(), 1);
    assert!(launches.ends_with(&format!("--resume\n{SESSION}\n")));
}

#[test]
fn reset_starts_a_new_conversation() {
    let claude = FakeClaude::printing("reset", &whole_reply(), "");
    let mut chat = claude.chat();

    reply_to(&mut chat, "Salut").0.unwrap();
    chat.reset();
    reply_to(&mut chat, "Nouveau").0.unwrap();

    assert!(!claude.read("arguments.txt").contains("--resume"));
}

#[test]
fn never_resumes_a_session_id_that_is_not_one() {
    let forged = INIT.replace(SESSION, "--dangerously-skip-permissions");
    let mut lines = whole_reply();
    lines[0] = &forged;
    let claude = FakeClaude::printing("forged", &lines, "");
    let mut chat = claude.chat();

    reply_to(&mut chat, "Salut").0.unwrap();
    reply_to(&mut chat, "Encore").0.unwrap();

    assert!(!claude.read("arguments.txt").contains("--resume"));
}

#[test]
fn reports_a_result_in_error() {
    let failed = r#"{"type":"result","subtype":"error","is_error":true,"api_error_status":429,"total_cost_usd":0}"#;
    let claude = FakeClaude::printing("error", &[INIT, failed], "");

    let (result, _) = reply_to(&mut claude.chat(), "Salut");

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::RateLimited);
}

#[test]
fn reports_a_declined_reply() {
    let refused = END.replace("end_turn", "refusal");
    let claude = FakeClaude::printing("declined", &[INIT, START, TEXT, &refused, RESULT], "");

    let (result, _) = reply_to(&mut claude.chat(), "Salut");

    assert_eq!(result.unwrap_err().failure(), ReplyFailure::Declined);
}

#[test]
fn reports_a_claude_code_that_stops_without_result() {
    let claude = FakeClaude::printing("silent", &[INIT], "exit 1");

    let error = reply_to(&mut claude.chat(), "Salut").0.unwrap_err();

    assert_eq!(error.failure(), ReplyFailure::ClaudeCodeFailed);
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn ends_at_the_result_without_waiting_for_a_lingering_child() {
    let claude = FakeClaude::printing("lingering", &whole_reply(), "sleep 30 &");
    let started = Instant::now();

    let (result, _) = reply_to(&mut claude.chat(), "Salut");

    assert!(result.is_ok());
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn stops_at_once_when_cancelled_mid_reply() {
    let claude = FakeClaude::printing("cancel", &[INIT, START, TEXT, OK], "sleep 30 & sleep 30");
    let mut chat = claude.chat();
    let cancellation = Cancellation::new();
    let (heard_sender, heard) = mpsc::channel();

    let (result, stopped_after) = thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let hands = Hands {
                toolbox: &FakeToolbox::without_folder(),
                permissions: &Answering::with(Decision::Deny),
                cancellation: &cancellation,
            };
            chat.reply(text("Salut"), &hands, &mut |event| {
                heard_sender.send(event).unwrap();
            })
        });
        heard.recv_timeout(Duration::from_secs(5)).unwrap();
        let cancelled_at = Instant::now();
        cancellation.cancel();
        (worker.join().unwrap(), cancelled_at.elapsed())
    });

    let error = result.unwrap_err();
    assert_eq!(error.failure(), ReplyFailure::Stopped);
    assert_eq!(error.spent().context_tokens, 2 + 649 + 1_680 + 1);
    assert!(error.spent().cost > Cost::default());
    assert!(stopped_after < Duration::from_secs(1));
}

#[test]
fn needs_claude_code_and_a_folder() {
    let without_program = ClaudeCodeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        None,
        Some(PathBuf::from("/tmp")),
    );
    let without_folder = ClaudeCodeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        Some(PathBuf::from("/usr/bin/true")),
        None,
    );

    for mut chat in [without_program, without_folder] {
        let error = reply_to(&mut chat, "Salut").0.unwrap_err();
        assert_eq!(error.failure(), ReplyFailure::ClaudeCodeFailed);
    }
}

#[test]
fn hands_the_images_over_with_the_text() {
    let claude = FakeClaude::printing("image", &whole_reply(), "");
    let png = b"\x89PNG\r\n\x1a\n pixels".to_vec();
    let prompt = Prompt {
        text: "Et ça ?".to_owned(),
        images: vec![dot_api::Image::from_bytes(png).unwrap()],
    };

    let hands = Hands {
        toolbox: &FakeToolbox::without_folder(),
        permissions: &Answering::with(Decision::Deny),
        cancellation: &Cancellation::new(),
    };
    claude.chat().reply(prompt, &hands, &mut |_| {}).unwrap();

    let handed: serde_json::Value = serde_json::from_str(&claude.read("prompt.txt")).unwrap();
    let content = &handed["message"]["content"];
    assert_eq!(content[0]["source"]["media_type"], "image/png");
    assert_eq!(content[1]["text"], "Et ça ?");
}

#[test]
fn reads_a_line_of_claude_code_up_to_sixteen_mebibytes() {
    let padding = " ".repeat(16 * 1_024 * 1_024 - 200);
    let padded = format!(
        r#"{{"type":"stream_event",{padding}"event":{{"type":"content_block_delta","index":1,"delta":{{"type":"text_delta","text":"ok"}}}}}}"#
    );
    let stream = [START, TEXT, &padded, RESULT].join("\n");
    let mut session = None;
    let mut stdin = None;
    let mut heard = Vec::new();
    let hands = Hands {
        toolbox: &FakeToolbox::without_folder(),
        permissions: &Answering::with(Decision::Deny),
        cancellation: &Cancellation::new(),
    };

    let listened = Listener {
        session: &mut session,
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |event| heard.push(event),
    }
    .listen(stream.as_bytes())
    .unwrap();

    assert_eq!(heard, [ReplyEvent::Text("ok".into())]);
    assert!(listened.outcome.is_some());
}

const TOOL_START: &str = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"Bash","input":{}}}}"#;
const TOOL_INPUT: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\":\"touch a.txt\"}"}}}"#;
const TOOL_STOP: &str =
    r#"{"type":"stream_event","event":{"type":"content_block_stop","index":0}}"#;
const ASKED: &str = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"touch a.txt"},"tool_use_id":"toolu_1"}}"#;
const RAN: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"","is_error":false}]},"parent_tool_use_id":null,"tool_use_result":{"stdout":""}}"#;
const REFUSED_RESULT: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"refused","is_error":true}]},"parent_tool_use_id":null}"#;

impl FakeClaude {
    fn asking(name: &str, before: &[&str], after: &[&str]) -> Self {
        let claude = Self::printing(name, &[], "");
        let printed =
            |lines: &[&str]| -> String { lines.iter().flat_map(|line| [*line, "\n"]).collect() };
        let script = format!(
            "#!/bin/sh\nIFS= read -r prompt\nprintf '%s\\n' \"$prompt\" > prompt.txt\ncat <<'EOF'\n{}{ASKED}\nEOF\nIFS= read -r answer\nprintf '%s\\n' \"$answer\" > answer.txt\ncat <<'EOF'\n{}EOF\n",
            printed(before),
            printed(after)
        );
        fs::write(claude.folder.join("bin/claude"), script).unwrap();
        claude
    }
}

#[test]
fn asks_the_user_before_claude_code_runs_a_command_and_hands_over_the_answer() {
    let claude = FakeClaude::asking(
        "allowed",
        &[INIT, START, TOOL_START, TOOL_INPUT, TOOL_STOP],
        &[RAN, RESULT],
    );
    let permissions = Answering::with(Decision::Allow);

    let (result, heard) = reply_with(&mut claude.chat(), "Crée a.txt", &permissions);

    result.unwrap();
    assert_eq!(
        permissions.asked(),
        [dot_api::Permission {
            tool_id: "toolu_1".to_owned(),
            action: dot_api::Action::Run {
                command: "touch a.txt".to_owned()
            },
        }]
    );
    let answer: serde_json::Value = serde_json::from_str(&claude.read("answer.txt")).unwrap();
    assert_eq!(
        answer,
        serde_json::json!({"type": "control_response", "response": {"subtype": "success",
            "request_id": "req-1",
            "response": {"behavior": "allow", "updatedInput": {"command": "touch a.txt"}}}})
    );
    assert_eq!(
        heard,
        [
            ReplyEvent::ToolStarted {
                id: "toolu_1".to_owned(),
                tool: ToolUse {
                    kind: ToolKind::Run,
                    subject: "touch a.txt".to_owned()
                },
            },
            ReplyEvent::ToolEnded {
                id: "toolu_1".to_owned(),
                end: ToolEnd::Done(Detail::Nothing),
            },
        ]
    );
}

#[test]
fn a_refusal_reaches_claude_code_and_shows_as_refused() {
    let claude = FakeClaude::asking(
        "refused",
        &[INIT, START, TOOL_START, TOOL_INPUT, TOOL_STOP],
        &[REFUSED_RESULT, RESULT],
    );

    let (result, heard) = reply_to(&mut claude.chat(), "Crée a.txt");

    result.unwrap();
    let answer: serde_json::Value = serde_json::from_str(&claude.read("answer.txt")).unwrap();
    assert_eq!(answer["response"]["response"]["behavior"], "deny");
    assert_eq!(
        answer["response"]["response"]["message"],
        dot_api::USER_REFUSED_THE_TOOL
    );
    assert_eq!(answer["response"]["response"]["interrupt"], false);
    assert_eq!(
        heard.last(),
        Some(&ReplyEvent::ToolEnded {
            id: "toolu_1".to_owned(),
            end: ToolEnd::Refused,
        })
    );
}

#[test]
fn runs_claude_code_in_the_project_folder() {
    let claude = FakeClaude::printing("project", &whole_reply(), "pwd -P > where.txt");
    let project = claude.folder.join("project");
    fs::create_dir_all(&project).unwrap();
    let hands = Hands {
        toolbox: &FakeToolbox::in_folder(project.to_str().unwrap()),
        permissions: &Answering::with(Decision::Deny),
        cancellation: &Cancellation::new(),
    };

    claude
        .chat()
        .reply(text("Salut"), &hands, &mut |_| {})
        .unwrap();

    let ran_in = fs::read_to_string(project.join("where.txt")).unwrap();
    assert_eq!(
        PathBuf::from(ran_in.trim()),
        fs::canonicalize(&project).unwrap()
    );
    assert!(!claude.workspace().exists());
}

#[test]
fn answers_a_request_dot_does_not_handle_with_an_error() {
    let other =
        r#"{"type":"control_request","request_id":"req-9","request":{"subtype":"hook_callback"}}"#;
    let claude = FakeClaude::asking("other", &[INIT], &[RESULT]);
    let script = fs::read_to_string(claude.folder.join("bin/claude"))
        .unwrap()
        .replace(ASKED, other);
    fs::write(claude.folder.join("bin/claude"), script).unwrap();
    let permissions = Answering::with(Decision::Allow);

    reply_with(&mut claude.chat(), "Salut", &permissions)
        .0
        .unwrap();

    assert_eq!(permissions.asked(), []);
    let answer: serde_json::Value = serde_json::from_str(&claude.read("answer.txt")).unwrap();
    assert_eq!(answer["response"]["subtype"], "error");
    assert_eq!(answer["response"]["request_id"], "req-9");
}
