use serde_json::json;

use super::*;

const SUB_AGENT_CALLS_BASH: &str = r#"{"type":"assistant","message":{"model":"claude-haiku-5-5","id":"msg_011","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01Fu","name":"Bash","input":{"command":"ls","description":"List files in current directory"},"caller":{"type":"direct"}}],"stop_reason":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":31845,"cache_read_input_tokens":0,"output_tokens":17}},"parent_tool_use_id":"toolu_01Ue","agent_id":"ace55","subagent_type":"general-purpose","task_description":"List files in current folder"}"#;

#[test]
fn reads_what_a_sub_agent_calls_and_writes_from_its_message() {
    let read = message(SUB_AGENT_CALLS_BASH.as_bytes()).unwrap();

    assert_eq!(
        read,
        AgentLine::Message(AgentMessage {
            tool: "toolu_01Ue".to_owned(),
            said: Vec::new(),
            calls: vec![AgentCall {
                id: "toolu_01Fu".to_owned(),
                name: "Bash".to_owned(),
                input: json!({"command": "ls", "description": "List files in current directory"}),
            }],
            tokens: 17,
        })
    );
}

#[test]
fn keeps_what_a_sub_agent_says_but_not_its_blank_text_nor_its_thinking() {
    let line =
        json!({"type": "assistant", "parent_tool_use_id": "toolu_1", "message": {"content": [
            {"type": "thinking", "thinking": "hmm"},
            {"type": "text", "text": "  \n"},
            {"type": "text", "text": "Found it."},
        ]}})
        .to_string();

    let AgentLine::Message(read) = message(line.as_bytes()).unwrap() else {
        panic!("not a message");
    };

    assert_eq!(read.said, ["Found it."]);
    assert_eq!(read.tokens, 0);
    assert_eq!(read.calls, []);
}

#[test]
fn refuses_a_message_that_names_no_agent() {
    let line = json!({"type": "assistant", "message": {"content": []}}).to_string();

    assert!(message(line.as_bytes()).is_err());
}

#[test]
fn reads_the_tokens_an_agent_handed_back_with_its_report() {
    let handed_back = json!({"status": "completed", "agentType": "Explore",
        "usage": {"input_tokens": 2, "output_tokens": 243}});
    let without_usage = json!({"agentType": "Explore"});
    let not_an_agent = json!({"usage": {"output_tokens": 243}});

    assert_eq!(tokens_handed_back(&handed_back), Some(243));
    assert_eq!(tokens_handed_back(&without_usage), None);
    assert_eq!(tokens_handed_back(&not_an_agent), None);
}
