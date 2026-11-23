use super::*;

const TASK_STARTED: &str = r#"{"type":"system","subtype":"task_started","task_id":"ace55","tool_use_id":"toolu_01Ue","description":"List files in current folder","subagent_type":"general-purpose","is_backgrounded":false,"prompt":"Run ls"}"#;
const TASK_NOTIFICATION: &str = r#"{"type":"system","subtype":"task_notification","task_id":"ace55","tool_use_id":"toolu_01Ue","status":"completed","summary":"The ls lists one file:\n\n- a.txt","usage":{"total_tokens":46055,"tool_uses":1,"duration_ms":4945}}"#;

#[test]
fn reads_an_agent_that_starts_with_its_kind() {
    let Line::Agent(started) = parse(TASK_STARTED.as_bytes()).unwrap() else {
        panic!("not about an agent");
    };

    assert_eq!(
        started,
        AgentLine::Started {
            tool: "toolu_01Ue".to_owned(),
            kind: "general-purpose".to_owned(),
        }
    );
}

#[test]
fn reads_the_report_of_an_agent_that_finished() {
    let Line::Agent(reported) = parse(TASK_NOTIFICATION.as_bytes()).unwrap() else {
        panic!("not about an agent");
    };

    assert_eq!(
        reported,
        AgentLine::Reported {
            tool: "toolu_01Ue".to_owned(),
            text: "The ls lists one file:\n\n- a.txt".to_owned(),
        }
    );
}

#[test]
fn a_task_line_that_names_no_call_is_only_about_the_session() {
    let line = r#"{"type":"system","subtype":"task_started","session_id":"x"}"#;

    assert!(matches!(
        parse(line.as_bytes()).unwrap(),
        Line::System {
            compaction: None,
            ..
        }
    ));
}

#[test]
fn reads_a_summary_only_on_a_compact_boundary() {
    let boundary = r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"pre_tokens":412000,"post_tokens":131000}}"#;
    let other = r#"{"type":"system","subtype":"status","compact_metadata":{"pre_tokens":1,"post_tokens":1}}"#;

    let Line::System { compaction, .. } = parse(boundary.as_bytes()).unwrap() else {
        panic!("not a system line");
    };
    let Line::System {
        compaction: none, ..
    } = parse(other.as_bytes()).unwrap()
    else {
        panic!("not a system line");
    };

    assert_eq!(
        compaction,
        Some(Compaction {
            before: 412_000,
            after: 131_000
        })
    );
    assert_eq!(none, None);
}
