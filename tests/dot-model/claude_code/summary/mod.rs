use super::*;

const SUMMARIZED: &str = r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"manual","pre_tokens":114145,"post_tokens":3797}}"#;
const STATUS: &str = r#"{"type":"system","subtype":"status","status":"compacting"}"#;
const RESULT: &str = r#"{"type":"result","is_error":false,"total_cost_usd":0.064}"#;

fn heard(lines: &[&str]) -> Option<Compaction> {
    heard_a_summary(lines.join("\n").as_bytes()).unwrap()
}

#[test]
fn hears_the_summary_claude_code_made() {
    assert_eq!(
        heard(&[STATUS, SUMMARIZED, RESULT]),
        Some(Compaction {
            before: 114_145,
            after: 3_797
        })
    );
}

#[test]
fn hears_no_summary_where_claude_code_made_none() {
    assert_eq!(heard(&[STATUS, RESULT]), None);
    assert_eq!(heard(&["Warning: not JSON", STATUS]), None);
    assert_eq!(heard(&[]), None);
}

#[test]
fn stops_listening_at_the_result() {
    assert_eq!(heard(&[RESULT, SUMMARIZED]), None);
}
