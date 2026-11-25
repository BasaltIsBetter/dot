use std::io::BufReader;
use std::time::Instant;

use super::*;
use crate::engine::tests::Scratch;
use crate::fleet::tests::Fake;

const ANSWERS: &str = r#"cat > /dev/null; echo '{"type":"result","result":"Done."}'"#;
const HANGS: &str = r#"case "$*" in *dot-end-call*) : > "$HERE/ended" ;; *) sleep 30 ;; esac"#;

#[derive(Clone, Default)]
struct Shared(std::sync::Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Shared {
    fn messages(&self) -> Vec<Value> {
        let written = self.0.lock().unwrap();
        written
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }
}

struct Exchange {
    fake: Fake,
    agents: Scratch,
}

impl Exchange {
    fn new(name: &str, exec: &str) -> Self {
        let agents = Scratch::new("agents");
        agents.write("reviewer.json", r#"{"description": "Reviews"}"#);
        Self {
            fake: Fake::new(name, exec),
            agents,
        }
    }

    fn talk(&self, until: impl Fn(&Shared) -> bool, messages: &[Value]) -> Vec<Value> {
        let (from, mut to) = io::pipe().unwrap();
        let heard = Shared::default();
        thread::scope(|scope| {
            let output = heard.clone();
            scope.spawn(move || {
                serve(
                    &self.fake.fleet,
                    self.agents.path(),
                    &self.fake.project,
                    (BufReader::new(from), output),
                    &Cancellation::new(),
                )
                .unwrap();
            });
            for message in messages {
                writeln!(to, "{message}").unwrap();
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            while !until(&heard) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            drop(to);
        });
        heard.messages()
    }
}

fn call(id: u32, agent: &str) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": "call_agent", "arguments": {"agent": agent, "prompt": "Look"}},
    })
}

#[test]
fn welcomes_claude_and_shows_its_tools() {
    let exchange = Exchange::new("mcp-welcome", ANSWERS);

    let answers = exchange.talk(
        |heard| heard.messages().len() == 3,
        &[
            json!({"id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
            json!({"method": "notifications/initialized"}),
            json!({"id": 1, "method": "tools/list"}),
            json!({"id": 2, "method": "resources/list"}),
        ],
    );

    assert_eq!(answers[0]["result"]["serverInfo"]["name"], "dot-agents");
    assert_eq!(answers[1]["result"]["tools"][1]["name"], "call_agent");
    assert_eq!(answers[2]["error"]["code"], METHOD_NOT_FOUND);
}

#[test]
fn calls_an_agent_and_hands_back_its_answer() {
    let exchange = Exchange::new("mcp-call", ANSWERS);

    let answers = exchange.talk(|heard| !heard.messages().is_empty(), &[call(5, "reviewer")]);

    assert_eq!(answers[0]["id"], 5);
    assert_eq!(answers[0]["result"]["isError"], false);
    assert_eq!(answers[0]["result"]["content"][0]["text"], "Done.");
}

#[test]
fn an_agent_nobody_described_is_not_called() {
    let exchange = Exchange::new("mcp-unknown", ANSWERS);

    let answers = exchange.talk(
        |heard| heard.messages().len() == 2,
        &[call(1, "stranger"), call(2, "../reviewer")],
    );

    assert!(
        answers
            .iter()
            .all(|answer| answer["result"]["isError"] == true)
    );
    assert_eq!(exchange.fake.runs_of("exec"), 0);
}

#[test]
fn lists_the_agents_and_what_cannot_be_used() {
    let exchange = Exchange::new("mcp-list", ANSWERS);
    exchange.agents.write("broken.json", "{");

    let answers = exchange.talk(
        |heard| !heard.messages().is_empty(),
        &[json!({"id": 1, "method": "tools/call", "params": {"name": "list_agents"}})],
    );

    let text = answers[0]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Reviews") && text.contains("broken.json"));
}

#[test]
fn the_end_of_the_exchange_ends_its_calls_in_their_container() {
    let exchange = Exchange::new("mcp-cancelled", HANGS);

    let answers = exchange.talk(
        |_| exchange.fake.runs_of("exec") > 0,
        &[call(1, "reviewer")],
    );

    assert_eq!(answers, [] as [Value; 0]);
    assert!(exchange.fake.scratch.path().join("ended").exists());
}

#[test]
fn a_call_claude_gave_up_is_ended_while_the_exchange_goes_on() {
    let exchange = Exchange::new("mcp-cancelled", HANGS);
    let ended = || exchange.fake.scratch.path().join("ended").exists();
    let asked = [call(1, "reviewer")];
    let (from, mut to) = io::pipe().unwrap();
    let heard = Shared::default();

    thread::scope(|scope| {
        let output = heard.clone();
        scope.spawn(|| {
            let messages = (BufReader::new(from), output);
            let fake = &exchange.fake;
            serve(
                &fake.fleet,
                exchange.agents.path(),
                &fake.project,
                messages,
                &Cancellation::new(),
            )
            .unwrap();
        });
        let wait = |done: &dyn Fn() -> bool| {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !done() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            assert!(done());
        };
        writeln!(to, "{}", asked[0]).unwrap();
        wait(&|| exchange.fake.runs_of("exec") > 0);
        let gave_up = json!({"method": "notifications/cancelled", "params": {"requestId": 1}});
        writeln!(to, "{gave_up}").unwrap();
        wait(&ended);
        writeln!(to, "{}", json!({"id": 2, "method": "ping"})).unwrap();
        wait(&|| !heard.messages().is_empty());
        drop(to);
    });

    let ids: Vec<Value> = heard
        .messages()
        .iter()
        .map(|told| told["id"].clone())
        .collect();
    assert_eq!(ids, [json!(2)]);
}

#[test]
fn what_is_no_message_is_told_and_does_not_end_the_exchange() {
    let exchange = Exchange::new("mcp-garbage", ANSWERS);

    let answers = exchange.talk(
        |heard| heard.messages().len() == 2,
        &[json!([1, 2]), json!({"id": 1, "method": "ping"})],
    );

    assert_eq!(answers[0]["error"]["code"], PARSE_ERROR);
    assert_eq!(answers[1]["result"], json!({}));
}
