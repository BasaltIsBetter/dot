use proptest::prelude::*;

use super::*;

fn line(json: &str) -> Line {
    parse(json.as_bytes()).unwrap()
}

#[test]
fn reads_the_session_of_the_init_line() {
    let Line::System { session_id, .. } = line(
        r#"{"type":"system","subtype":"init","session_id":"3898f4c9-b1b3-480e-b180-b139f7eb2054","tools":[]}"#,
    ) else {
        panic!("not a system line");
    };

    assert_eq!(
        session_id.as_deref(),
        Some("3898f4c9-b1b3-480e-b180-b139f7eb2054")
    );
}

fn compaction_of(json: &str) -> Option<Compaction> {
    let Line::System { compaction, .. } = line(json) else {
        panic!("not a system line");
    };
    compaction
}

#[test]
fn reads_how_far_a_summary_shrank_the_conversation() {
    let compaction = compaction_of(
        r#"{"type":"system","subtype":"compact_boundary","uuid":"3491fbf7-149a-46db-acfc-f412c4bd8120","compact_metadata":{"trigger":"auto","pre_tokens":154908,"post_tokens":16764,"cumulative_dropped_tokens":138144,"duration_ms":7369}}"#,
    );

    assert_eq!(
        compaction,
        Some(Compaction {
            before: 154_908,
            after: 16_764,
        })
    );
}

#[test]
fn takes_no_other_system_line_for_a_summary() {
    let sizes = r#""compact_metadata":{"pre_tokens":9,"post_tokens":3}"#;

    assert_eq!(
        compaction_of(&format!(r#"{{"type":"system","subtype":"init",{sizes}}}"#)),
        None
    );
    assert_eq!(
        compaction_of(&format!(r#"{{"type":"system",{sizes}}}"#)),
        None
    );
    assert_eq!(
        compaction_of(r#"{"type":"system","subtype":"compact_boundary"}"#),
        None
    );
}

#[test]
fn refuses_a_summary_that_does_not_give_both_sizes() {
    for sizes in [
        r#"{"pre_tokens":9}"#,
        r#"{"pre_tokens":-1,"post_tokens":3}"#,
    ] {
        let line = format!(
            r#"{{"type":"system","subtype":"compact_boundary","compact_metadata":{sizes}}}"#
        );

        assert!(parse(line.as_bytes()).is_err(), "{line}");
    }
}

#[test]
fn unwraps_the_api_event_of_a_stream_line() {
    let parsed = line(
        r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}},"session_id":"x"}"#,
    );

    assert!(matches!(
        parsed,
        Line::Stream(Event::ContentBlockDelta { .. })
    ));
}

#[test]
fn reads_the_plan_limits_in_percent() {
    let Line::RateLimits(limits) = line(
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed",
            "unifiedWindows":{"seven_day":{"utilization":0.47,"status":"allowed"},
                              "five_hour":{"utilization":0.031}}}}"#,
    ) else {
        panic!("not a rate limit line");
    };

    assert_eq!(limits.seven_days, Some(47));
    assert_eq!(limits.five_hours, Some(3));
}

#[test]
fn knows_no_limit_claude_code_does_not_report() {
    let Line::RateLimits(limits) =
        line(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#)
    else {
        panic!("not a rate limit line");
    };

    assert_eq!(limits, Limits::default());
}

#[test]
fn keeps_shares_within_zero_and_a_hundred_percent() {
    let percent_of = |utilization: f64| percent(Window { utilization });

    assert_eq!(percent_of(1.7), 100);
    assert_eq!(percent_of(-0.2), 0);
    assert_eq!(percent_of(f64::NAN), 0);
    assert_eq!(percent_of(f64::INFINITY), 0);
    assert_eq!(percent_of(0.995), 100);
}

#[test]
fn reads_the_result_wherever_its_type_stands() {
    let Line::Result(outcome) = line(
        r#"{"duration_api_ms":1118,"total_cost_usd":0.0136578,"is_error":false,"type":"result","subtype":"success"}"#,
    ) else {
        panic!("not a result line");
    };

    assert_eq!(outcome.failure(), None);
    assert_eq!(outcome.cost(), Cost::from_micro_dollars(13_658));
}

#[test]
fn tells_each_failed_result_apart() {
    let failure_of = |status: Option<u16>| {
        Outcome {
            is_error: Some(true),
            total_cost_usd: Some(0.0),
            api_error_status: status,
        }
        .failure()
    };

    assert_eq!(failure_of(Some(401)), Some(ReplyFailure::Unauthorized));
    assert_eq!(failure_of(Some(403)), Some(ReplyFailure::Unauthorized));
    assert_eq!(failure_of(Some(429)), Some(ReplyFailure::RateLimited));
    assert_eq!(failure_of(Some(529)), Some(ReplyFailure::Overloaded));
    assert_eq!(failure_of(Some(503)), Some(ReplyFailure::Overloaded));
    assert_eq!(failure_of(None), Some(ReplyFailure::ClaudeCodeFailed));
}

#[test]
fn counts_an_impossible_cost_as_nothing() {
    assert_eq!(micro_dollars(-1.0), 0);
    assert_eq!(micro_dollars(f64::NAN), 0);
    assert_eq!(micro_dollars(f64::INFINITY), 0);
    assert_eq!(micro_dollars(0.000_000_4), 0);
    assert_eq!(micro_dollars(2.4), 2_400_000);
}

#[test]
fn passes_over_what_a_sub_agent_streams() {
    let sub_agent =
        r#"{"type":"stream_event","parent_tool_use_id":"toolu_1","event":{"type":"message_stop"}}"#;

    assert!(matches!(line(sub_agent), Line::Other));
}

#[test]
fn reads_the_tool_results_claude_code_sends_back() {
    let user =
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1"}]}}"#;

    let Line::ToolResults { agent, results } = line(user) else {
        panic!("not tool results");
    };
    assert_eq!(agent, None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "toolu_1");
}

#[test]
fn reads_a_permission_request() {
    let asked = r#"{"type":"control_request","request_id":"req-7","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"toolu_1"}}"#;

    let Line::Control(request) = line(asked) else {
        panic!("not a control request");
    };
    assert_eq!(request.request_id, "req-7");
}

#[test]
fn passes_over_the_lines_it_does_not_need() {
    assert!(matches!(
        line(r#"{"type":"assistant","message":{"content":[{"type":"text","text":"ok"}]}}"#),
        Line::Other
    ));
    assert!(parse(b"Warning: no stdin data received in 3s").is_err());
}

proptest! {
    #[test]
    fn reads_anything_without_panicking(bytes in proptest::collection::vec(any::<u8>(), 0..300)) {
        if let Ok(Line::RateLimits(limits)) = parse(&bytes) {
            prop_assert!(limits.seven_days.is_none_or(|share| share <= 100));
        }
    }
}

#[test]
fn reads_the_message_of_a_sub_agent_and_skips_those_of_claude() {
    let of_an_agent = r#"{"type":"assistant","parent_tool_use_id":"toolu_1","message":{"content":[{"type":"text","text":"Found it."}],"usage":{"output_tokens":5}}}"#;
    let of_claude = r#"{"type":"assistant","parent_tool_use_id":null,"message":{"content":[{"type":"text","text":"ok"}]}}"#;

    assert!(matches!(
        line(of_an_agent),
        Line::Agent(AgentLine::Message(_))
    ));
    assert!(matches!(line(of_claude), Line::Other));
}

#[test]
fn reads_a_request_it_cannot_understand_as_one_to_answer_with_an_error() {
    let asked = r#"{"type":"control_request","request_id":9,"request":{"tool_name":7}}"#;

    let Line::Control(request) = line(asked) else {
        panic!("not a control request");
    };
    let mut written = Vec::new();
    request.answer(None, &mut written).unwrap();

    assert_eq!(request.request_id, serde_json::json!(9));
    let answered: Value = serde_json::from_slice(&written).unwrap();
    assert_eq!(answered["response"]["subtype"], "error");
    assert_eq!(answered["response"]["request_id"], serde_json::json!(9));
}

#[test]
fn passes_over_a_request_without_an_id() {
    let asked = r#"{"type":"control_request","request":{"subtype":"can_use_tool"}}"#;

    assert!(matches!(line(asked), Line::Other));
}

#[test]
fn reads_a_result_whose_fields_are_null_as_one_that_went_well_at_no_cost() {
    let Line::Result(outcome) =
        line(r#"{"type":"result","subtype":"success","is_error":null,"total_cost_usd":null}"#)
    else {
        panic!("not a result");
    };

    assert_eq!(outcome.failure(), None);
    assert_eq!(outcome.cost(), Cost::default());
}
