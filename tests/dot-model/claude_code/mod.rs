use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use dot_api::{Cancellation, Cost, Decision, Detail, ToolEnd, ToolKind, ToolUse};

use super::*;
use crate::claude_code::listen::tests::arrivals;
use crate::tool::tests::{Answering, FakeToolbox, Untouched};

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

pub(crate) struct FakeClaude {
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
        Self::running(name, &script)
    }

    pub(crate) fn running(name: &str, script: &str) -> Self {
        let folder = std::env::temp_dir().join(format!("dot-claude-{}-{name}", std::process::id()));
        if folder.exists() {
            fs::remove_dir_all(&folder).unwrap();
        }
        fs::create_dir_all(folder.join("bin")).unwrap();
        let program = folder.join("bin/claude");
        fs::write(&program, script).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        Self { folder }
    }

    fn chat(&self) -> ClaudeCodeChat {
        ClaudeCodeChat::new(
            Model::OPUS_5_5,
            Effort::High,
            Some(self.program()),
            Some(self.workspace()),
            None,
        )
    }

    pub(crate) fn program(&self) -> PathBuf {
        self.folder.join("bin/claude")
    }

    pub(crate) fn workspace(&self) -> PathBuf {
        self.folder.join("Application Support/Dot")
    }

    pub(crate) fn read(&self, name: &str) -> String {
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

pub(crate) const FAKE_LOGIN_PATH: &str = "/opt/dot-test/bin:/usr/bin:/bin";

impl FakeClaude {
    pub(crate) fn telling_its_path(name: &str) -> Self {
        let printed = whole_reply().join("\n");
        let script = format!(
            "#!/bin/sh\nprintenv PATH > path.txt\nIFS= read -r prompt\ncat <<'EOF'\n{printed}\nEOF\n"
        );
        Self::running(name, &script)
    }

    pub(crate) fn login_path(&self) -> LoginPath {
        let shell = self.folder.join("bin/login-shell");
        let script = format!("#!/bin/sh\nPATH={FAKE_LOGIN_PATH} exec /bin/sh -c \"$4\"\n");
        fs::write(&shell, script).unwrap();
        fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();
        LoginPath::new(Some(shell))
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
        toolbox: &mut FakeToolbox::without_folder(),
        permissions,
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
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
    assert_eq!(reply.output_tokens, 4);
    assert_eq!(reply.limits.seven_days, Some(50));
}

#[test]
fn launches_claude_code_with_the_path_of_the_login_shell() {
    let claude = FakeClaude::telling_its_path("login-path");
    let mut chat = claude.chat().login_path(claude.login_path());

    let (result, _) = reply_to(&mut chat, "Salut");

    result.unwrap();
    assert_eq!(claude.read("path.txt"), format!("{FAKE_LOGIN_PATH}\n"));
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
fn only_the_reply_that_starts_a_conversation_reports_its_base() {
    let claude = FakeClaude::printing("base", &whole_reply(), "");
    let mut chat = claude.chat();

    let first = reply_to(&mut chat, "Salut").0.unwrap();
    let second = reply_to(&mut chat, "Encore").0.unwrap();

    assert_eq!(first.base_tokens, Some(2 + 649 + 1_680));
    assert_eq!(second.base_tokens, None);
}

fn windows_handed_over(claude: &FakeClaude) -> Vec<String> {
    let launches = claude.read("arguments.txt");
    let mut arguments = launches.lines();
    let mut windows = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--autocompact" {
            windows.push(arguments.next().unwrap_or_default().to_owned());
        }
    }
    windows
}

#[test]
fn hands_a_window_over_from_the_eleventh_request_of_a_conversation_when_asked_to() {
    let claude = FakeClaude::printing("window", &whole_reply(), "");
    let mut chat = claude.chat().summarizing(Summaries::WhenTheyPay);

    for _ in 0..10 {
        reply_to(&mut chat, "Salut").0.unwrap();
    }
    assert_eq!(windows_handed_over(&claude), [""; 0]);
    reply_to(&mut chat, "Encore").0.unwrap();
    assert_eq!(windows_handed_over(&claude), ["160913"]);

    chat.reset();
    reply_to(&mut chat, "Salut").0.unwrap();
    assert_eq!(windows_handed_over(&claude), ["160913"]);
}

#[test]
fn leaves_summaries_to_claude_unless_asked_otherwise() {
    let claude = FakeClaude::printing("no-window", &whole_reply(), "");
    let mut chat = claude.chat();

    for _ in 0..12 {
        reply_to(&mut chat, "Salut").0.unwrap();
    }

    assert_eq!(windows_handed_over(&claude), [""; 0]);
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
                toolbox: &mut FakeToolbox::without_folder(),
                permissions: &Answering::with(Decision::Deny),
                cancellation: &cancellation,
                compactor: &Untouched,
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
        None,
    );
    let without_folder = ClaudeCodeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        Some(PathBuf::from("/usr/bin/true")),
        None,
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
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &Answering::with(Decision::Deny),
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
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
        toolbox: &mut FakeToolbox::without_folder(),
        permissions: &Answering::with(Decision::Deny),
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
    };

    let listened = Listener {
        session: &mut session,
        ledger: &mut Ledger::NOTHING_YET.clone(),
        stdin: &mut stdin,
        hands: &hands,
        on_event: &mut |event| heard.push(event),
    }
    .listen(&arrivals(stream.as_bytes()))
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
        toolbox: &mut FakeToolbox::in_folder(project.to_str().unwrap()),
        permissions: &Answering::with(Decision::Deny),
        cancellation: &Cancellation::new(),
        compactor: &Untouched,
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
const COMPACTED: &str = r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"manual","pre_tokens":401000,"post_tokens":8000}}"#;
pub(crate) const REPLIES_OF_A_LONG_CONVERSATION: usize = 25;

pub(crate) fn long_conversation(name: &str) -> FakeClaude {
    let script = format!(
        r#"#!/bin/sh
IFS= read -r prompt
printf '%s\n' "$prompt" >> prompts.txt
printf '%s\n' "$@" >> arguments.txt
printf 'launch\n' >> launches.txt
case "$prompt" in
*'"/compact"'*)
  : > summarized
  cat <<'END'
{COMPACTED}
{RESULT}
END
  exit 0 ;;
esac
read=400000
[ "$(($(wc -l < launches.txt)))" -eq 1 ] && read=100000
[ -e summarized ] && read=150000
cat <<END
{INIT}
{{"type":"stream_event","event":{{"type":"message_start","message":{{"model":"claude-opus-5-5","usage":{{"input_tokens":$read,"cache_creation_input_tokens":1000,"cache_creation":{{"ephemeral_1h_input_tokens":1000}},"output_tokens":1}}}}}}}}
{RESULT}
END
"#
    );
    FakeClaude::running(name, &script)
}

fn grown_long(claude: &FakeClaude, summaries: Summaries) -> ClaudeCodeChat {
    let mut chat = claude.chat().summarizing(summaries);
    for _ in 0..REPLIES_OF_A_LONG_CONVERSATION {
        reply_to(&mut chat, "Salut").0.unwrap();
    }
    chat
}

fn launches(claude: &FakeClaude) -> usize {
    claude.read("launches.txt").lines().count()
}

#[test]
fn has_a_long_conversation_summarized_between_two_prompts() {
    let claude = long_conversation("settle");
    let mut chat = grown_long(&claude, Summaries::WhenTheyPay);

    chat.settle(None, &Cancellation::new());

    assert_eq!(launches(&claude), REPLIES_OF_A_LONG_CONVERSATION + 1);
    let prompts = claude.read("prompts.txt");
    assert!(
        prompts
            .lines()
            .last()
            .unwrap()
            .contains(r#""content":"/compact""#),
        "{prompts}"
    );

    let windows_before = windows_handed_over(&claude).len();
    reply_to(&mut chat, "Encore").0.unwrap();
    assert_eq!(windows_handed_over(&claude).len(), windows_before);
    reply_to(&mut chat, "Encore").0.unwrap();
    let window: u64 = windows_handed_over(&claude)
        .last()
        .unwrap()
        .parse()
        .unwrap();
    assert!(window >= 151_000 * 5 / 4 + 33_000, "{window}");
    chat.settle(None, &Cancellation::new());
    assert_eq!(launches(&claude), REPLIES_OF_A_LONG_CONVERSATION + 3);
}

#[test]
fn announces_a_summary_made_between_two_prompts_once_in_the_next_reply() {
    let claude = long_conversation("announced");
    let mut chat = grown_long(&claude, Summaries::WhenTheyPay);
    chat.settle(None, &Cancellation::new());

    let (first, heard_first) = reply_to(&mut chat, "Encore");
    let (second, heard_second) = reply_to(&mut chat, "Encore");

    first.unwrap();
    second.unwrap();
    let summary = ReplyEvent::Compacted {
        before: 401_000,
        after: 8_000,
    };
    assert_eq!(heard_first, [summary]);
    assert!(heard_second.is_empty(), "{heard_second:?}");
}

#[test]
fn forgets_an_unannounced_summary_when_the_conversation_starts_over() {
    let claude = long_conversation("forgotten");
    let mut chat = grown_long(&claude, Summaries::WhenTheyPay);
    chat.settle(None, &Cancellation::new());

    chat.reset();
    let (result, heard) = reply_to(&mut chat, "Salut");

    result.unwrap();
    assert!(heard.is_empty(), "{heard:?}");
}

#[test]
fn leaves_a_conversation_alone_until_a_summary_has_paid_for_itself() {
    let claude = long_conversation("not-due");
    let mut chat = claude.chat().summarizing(Summaries::WhenTheyPay);
    for _ in 0..12 {
        reply_to(&mut chat, "Salut").0.unwrap();
    }

    chat.settle(None, &Cancellation::new());

    assert_eq!(launches(&claude), 12);
}

#[test]
fn summarizes_nothing_between_two_prompts_unless_asked_to() {
    let claude = long_conversation("left-to-claude");
    let mut chat = grown_long(&claude, Summaries::LeftToClaude);

    chat.settle(None, &Cancellation::new());

    assert_eq!(launches(&claude), REPLIES_OF_A_LONG_CONVERSATION);
}

#[test]
fn a_summary_stopped_at_once_is_left_to_the_next_reply() {
    let claude = long_conversation("stopped");
    let mut chat = grown_long(&claude, Summaries::WhenTheyPay);
    let stopped = Cancellation::new();
    stopped.cancel();

    chat.settle(None, &stopped);
    reply_to(&mut chat, "Encore").0.unwrap();

    let window: u64 = windows_handed_over(&claude)
        .last()
        .unwrap()
        .parse()
        .unwrap();
    assert!(window <= 401_000, "{window}");
}

#[test]
fn counts_each_reply_once_though_claude_code_reports_the_total_of_the_conversation() {
    let script = format!(
        r#"#!/bin/sh
IFS= read -r prompt
printf 'launch\n' >> launches.txt
case "$(($(wc -l < launches.txt)))" in
1) total=0.05 ;;
2) total=0.08 ;;
*) total=0.02 ;;
esac
cat <<END
{INIT}
{START}
{{"type":"result","is_error":false,"total_cost_usd":$total}}
END
"#
    );
    let claude = FakeClaude::running("billed", &script);
    let mut chat = claude.chat();
    let cost_of_a_reply =
        |chat: &mut ClaudeCodeChat| reply_to(chat, "Salut").0.unwrap().cost.micro_dollars();

    let first = cost_of_a_reply(&mut chat);
    let second = cost_of_a_reply(&mut chat);
    let after_a_lower_total = cost_of_a_reply(&mut chat);
    chat.reset();
    let afresh = cost_of_a_reply(&mut chat);

    assert_eq!(
        (first, second, after_a_lower_total, afresh),
        (50_000, 30_000, 20_000, 20_000)
    );
}
const HOUSE_RULES: &str = "# House rules\n\nEvery answer ends with `-- checked`.\n";

fn with_house_rules(claude: &FakeClaude) {
    fs::create_dir_all(claude.workspace()).unwrap();
    fs::write(claude.workspace().join("CLAUDE.md"), HOUSE_RULES).unwrap();
}

#[test]
fn hands_the_instructions_of_its_folder_over_at_every_launch_when_asked_to() {
    let claude = long_conversation("handed");
    with_house_rules(&claude);
    let mut chat =
        grown_long(&claude, Summaries::WhenTheyPay).instructions(Instructions::HandedByDot);

    reply_to(&mut chat, "Salut").0.unwrap();
    chat.settle(None, &Cancellation::new());

    let arguments = claude.read("arguments.txt");
    assert_eq!(arguments.matches("--settings\n").count(), 2, "{arguments}");
    assert_eq!(
        arguments
            .matches("Every answer ends with `-- checked`.")
            .count(),
        2
    );
    assert_eq!(launches(&claude), REPLIES_OF_A_LONG_CONVERSATION + 2);
}

#[test]
fn launches_with_the_largest_instructions_it_hands_over() {
    let claude = long_conversation("largest-instructions");
    fs::create_dir_all(claude.workspace()).unwrap();
    fs::write(
        claude.workspace().join("CLAUDE.md"),
        vec![b'a'; usize::try_from(crate::claude_code::launch::LARGEST_INSTRUCTIONS).unwrap()],
    )
    .unwrap();
    let mut chat = claude.chat().instructions(Instructions::HandedByDot);

    reply_to(&mut chat, "Salut").0.unwrap();

    assert!(claude.read("arguments.txt").contains("--settings"));
}

#[test]
fn leaves_the_instructions_to_claude_code_unless_asked_otherwise() {
    let claude = long_conversation("not-handed");
    with_house_rules(&claude);
    let mut chat = claude.chat();

    reply_to(&mut chat, "Salut").0.unwrap();

    assert!(!claude.read("arguments.txt").contains("--settings"));
}

#[test]
fn hands_nothing_over_from_a_folder_without_instructions() {
    let claude = long_conversation("no-instructions");
    let mut chat = claude.chat().instructions(Instructions::HandedByDot);

    reply_to(&mut chat, "Salut").0.unwrap();

    assert!(!claude.read("arguments.txt").contains("--settings"));
}

#[test]
fn shows_what_a_command_of_claude_code_prints_while_it_runs() {
    let first = [INIT, START, TOOL_START, TOOL_INPUT, TOOL_STOP]
        .iter()
        .flat_map(|line| [*line, "\n"])
        .collect::<String>();
    let then = [RAN, RESULT]
        .iter()
        .flat_map(|line| [*line, "\n"])
        .collect::<String>();
    let script = format!(
        "#!/bin/sh\nIFS= read -r prompt\nprintenv DOT_LIVE_OUTPUT > live.txt\ncat <<'EOF'\n{first}EOF\ni=0\nwhile [ ! -f sent.txt ] && [ $i -lt 300 ]; do i=$((i+1)); sleep 0.01; done\nsleep 0.5\ncat <<'EOF'\n{then}EOF\n"
    );
    let claude = FakeClaude::running("live", &script);
    let workspace = PathBuf::from(format!("/tmp/dot-l{}", std::process::id()));
    let mut chat = ClaudeCodeChat::new(
        Model::OPUS_5_5,
        Effort::High,
        Some(claude.program()),
        Some(workspace.clone()),
        Some(PathBuf::from("/bin/sh")),
    );
    let address_file = workspace.join("live.txt");

    let (result, heard) = thread::scope(|scope| {
        scope.spawn(|| {
            let started = Instant::now();
            let address = loop {
                if let Ok(written) = fs::read_to_string(&address_file)
                    && written.ends_with('\n')
                {
                    break PathBuf::from(written.trim_end());
                }
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "no live address"
                );
                thread::sleep(Duration::from_millis(10));
            };
            thread::sleep(Duration::from_millis(300));
            let mut live = dot_process::LiveOutput::connect(&address, "touch a.txt").unwrap();
            live.send(b"\x1b[32mtouched\x1b[0m a.txt\n");
            fs::write(workspace.join("sent.txt"), "").unwrap();
        });
        reply_to(&mut chat, "Salut")
    });

    result.unwrap();
    let shown: Vec<&ReplyEvent> = heard
        .iter()
        .filter(|event| matches!(event, ReplyEvent::ToolOutput { .. }))
        .collect();
    assert_eq!(
        shown,
        [&ReplyEvent::ToolOutput {
            id: "toolu_1".to_owned(),
            lines: vec!["\x1b[32mtouched\x1b[0m a.txt".to_owned()],
        }]
    );
    let left = workspace.join("live").read_dir().unwrap().count();
    fs::remove_dir_all(&workspace).unwrap();
    assert_eq!(left, 0, "the socket is removed once the reply ends");
}

#[test]
fn says_the_last_thing_claude_code_wrote_when_it_stops_without_result() {
    let claude = FakeClaude::printing(
        "said",
        &[],
        "echo 'starting' >&2; echo \"error: unknown option '--autocompact'\" >&2; echo >&2; exit 1",
    );

    let error = reply_to(&mut claude.chat(), "Salut").0.unwrap_err();

    assert_eq!(error.failure(), ReplyFailure::ClaudeCodeFailed);
    let cause = std::error::Error::source(&error).unwrap().to_string();
    assert!(
        cause.ends_with(": error: unknown option '--autocompact'"),
        "{cause}"
    );
}

#[test]
fn forgets_a_session_claude_code_could_not_take_up_again() {
    let marker = std::env::temp_dir().join(format!("dot-claude-once-{}", std::process::id()));
    drop(fs::remove_file(&marker));
    let printed: String = whole_reply()
        .iter()
        .flat_map(|line| [*line, "\n"])
        .collect();
    let script = format!(
        "#!/bin/sh\nIFS= read -r prompt\nif [ -f '{}' ]; then echo 'No conversation found' >&2; exit 1; fi\ntouch '{}'\ncat <<'EOF'\n{printed}EOF\n",
        marker.display(),
        marker.display()
    );
    let claude = FakeClaude::running("resume", &script);
    let mut chat = claude.chat();

    assert!(reply_to(&mut chat, "Salut").0.is_ok());
    assert!(chat.session.is_some());
    let failed = reply_to(&mut chat, "Encore").0;

    assert!(failed.is_err());
    assert!(chat.session.is_none());
    drop(fs::remove_file(&marker));
}

#[test]
fn keeps_the_session_when_claude_code_stops_after_saying_something() {
    let claude = FakeClaude::printing("keeps", &[INIT, START, TEXT], "exit 1");
    let mut chat = claude.chat();

    let failed = reply_to(&mut chat, "Salut").0;

    assert!(failed.is_err());
    assert!(chat.session.is_some());
}

#[test]
fn hands_the_stand_in_the_compaction_tuned_last() {
    let quiet = OutputCompaction {
        rewriting: dot_api::Toggle::Off,
        ..OutputCompaction::FULL
    };
    let mut chat =
        ClaudeCodeChat::new(Model::OPUS_5_5, Effort::High, None, None, None).compacting(quiet);
    assert_eq!(chat.options((None, None), None, None).compaction, quiet);

    chat.tune(Tuning {
        model: Model::OPUS_5_5,
        effort: Effort::High,
        summaries: Summaries::LeftToClaude,
        compaction: OutputCompaction::FULL,
    });

    assert_eq!(
        chat.options((None, None), None, None).compaction,
        OutputCompaction::FULL
    );
}

#[test]
fn resumes_a_session_taken_up_from_a_memory_and_bills_only_what_is_new() {
    let claude = FakeClaude::printing("taken-up", &whole_reply(), "");
    let whole = reply_to(&mut claude.chat(), "Salut").0.unwrap().cost;
    let mut chat = claude.chat();

    chat.take_up(&serde_json::json!({
        "session": SESSION,
        "billed": whole.micro_dollars() - 100,
    }));
    let reply = reply_to(&mut chat, "Encore").0.unwrap();

    assert!(
        claude
            .read("arguments.txt")
            .ends_with(&format!("--resume\n{SESSION}\n"))
    );
    assert_eq!(reply.cost, Cost::from_micro_dollars(100));
}
