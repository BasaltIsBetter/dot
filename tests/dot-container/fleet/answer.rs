use proptest::prelude::*;

use super::*;

fn heard(is_claude: bool, lines: &[&str]) -> (Vec<CallEvent>, Option<Result<Answer, String>>) {
    let mut reader = Reader::of(is_claude);
    let mut events = Vec::new();
    for line in lines {
        reader.hear(line.as_bytes(), &mut |event| events.push(event));
    }
    (events, reader.answer())
}

#[test]
fn tells_what_claude_does_then_what_it_answers() {
    let (events, answer) = heard(
        true,
        &[
            r#"{"type":"system","subtype":"init","session_id":"s"}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"I look."},{"type":"tool_use","name":"Read","input":{}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result"}]}}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Done.","total_cost_usd":0.25}"#,
        ],
    );

    assert_eq!(
        events,
        [
            CallEvent::Text("I look.".to_owned()),
            CallEvent::Tool("Read".to_owned())
        ]
    );
    let answer = answer.unwrap().unwrap();
    assert_eq!(answer.text, "Done.");
    assert_eq!(answer.cost, Some(0.25));
}

#[test]
fn tells_what_claude_says_of_its_failure() {
    let (_, answer) = heard(
        true,
        &[r#"{"type":"result","is_error":true,"result":"Please sign in"}"#],
    );

    assert_eq!(answer, Some(Err("Please sign in".to_owned())));
}

#[test]
fn claude_that_ends_without_a_word_has_no_answer() {
    assert_eq!(heard(true, &["not json", "", "{}"]), (vec![], None));
}

#[test]
fn a_program_answers_with_what_it_prints() {
    let (events, answer) = heard(false, &["one", "two"]);

    assert_eq!(events.len(), 2);
    assert_eq!(answer.unwrap().unwrap().text, "one\ntwo");
}

#[test]
fn an_answer_never_floods_whoever_reads_it() {
    let long = "é".repeat(LONGEST_ANSWER);

    let (_, answer) = heard(false, &[&long]);

    let text = answer.unwrap().unwrap().text;
    assert!(text.len() <= LONGEST_ANSWER + CUT.len());
    assert!(text.ends_with(CUT));
}

proptest! {
    #[test]
    fn hears_anything_without_panicking(line in proptest::collection::vec(any::<u8>(), 0..200)) {
        for is_claude in [true, false] {
            let mut reader = Reader::of(is_claude);
            reader.hear(&line, &mut |_| {});
            drop(reader.answer());
        }
    }
}
