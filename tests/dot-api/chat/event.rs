use serde_json::{from_str, to_string};

use super::*;
use crate::file::{Hunk, PatchLine, Siblings};
use crate::tool::{Detail, ToolKind};

#[test]
fn keeps_each_event_smaller_than_the_change_of_a_file() {
    assert!(
        size_of::<ReplyEvent>() < size_of::<FileChange>(),
        "{} {}",
        size_of::<ReplyEvent>(),
        size_of::<FileChange>()
    );
}

fn written_as(event: &ReplyEvent, written: &str) {
    assert_eq!(to_string(event).expect("an event serializes"), written);
    assert_eq!(
        &from_str::<ReplyEvent>(written).expect("it reads back"),
        event
    );
}

#[test]
fn keeps_pieces_of_text_under_stable_names() {
    written_as(&ReplyEvent::Text("Hi".to_owned()), r#"{"text":"Hi"}"#);
    written_as(
        &ReplyEvent::Thinking("Hmm".to_owned()),
        r#"{"thinking":"Hmm"}"#,
    );
}

#[test]
fn keeps_tools_under_stable_names() {
    written_as(
        &ReplyEvent::ToolStarted {
            id: "t1".to_owned(),
            tool: ToolUse {
                kind: ToolKind::FindFiles,
                subject: "*.rs".to_owned(),
            },
        },
        r#"{"tool-started":{"id":"t1","tool":{"kind":"find-files","subject":"*.rs"}}}"#,
    );
    written_as(
        &ReplyEvent::ToolEnded {
            id: "t1".to_owned(),
            end: ToolEnd::Done(Detail::Changes {
                added: 2,
                removed: 1,
            }),
        },
        r#"{"tool-ended":{"id":"t1","end":{"done":{"changes":{"added":2,"removed":1}}}}}"#,
    );
    written_as(
        &ReplyEvent::ToolEnded {
            id: "t2".to_owned(),
            end: ToolEnd::Refused,
        },
        r#"{"tool-ended":{"id":"t2","end":"refused"}}"#,
    );
    written_as(
        &ReplyEvent::ToolOutput {
            id: "t3".to_owned(),
            lines: vec!["ok".to_owned()],
        },
        r#"{"tool-output":{"id":"t3","lines":["ok"]}}"#,
    );
}

#[test]
fn keeps_a_change_to_a_file_under_stable_names() {
    let change = FileChange {
        path: "a.rs".to_owned(),
        created: true,
        hunks: vec![Hunk {
            old_start: 0,
            new_start: 1,
            lines: vec![
                PatchLine::Added("b".to_owned()),
                PatchLine::Same("c".to_owned()),
                PatchLine::Removed("d".to_owned()),
            ],
        }],
        siblings: Some(Siblings {
            folder: "src/".to_owned(),
            before: vec!["lib.rs".to_owned()],
            after: Vec::new(),
        }),
    };

    written_as(
        &ReplyEvent::ToolChanged {
            id: "t1".to_owned(),
            change: Box::new(change),
        },
        concat!(
            r#"{"tool-changed":{"id":"t1","change":{"path":"a.rs","created":true,"hunks":"#,
            r#"[{"old-start":0,"new-start":1,"lines":[{"added":"b"},{"same":"c"},{"removed":"d"}]}],"#,
            r#""siblings":{"folder":"src/","before":["lib.rs"],"after":[]}}}}"#
        ),
    );
}

#[test]
fn keeps_agents_and_summaries_under_stable_names() {
    written_as(
        &ReplyEvent::Agent {
            id: "a1".to_owned(),
            event: AgentEvent::Started {
                kind: "Explore".to_owned(),
            },
        },
        r#"{"agent":{"id":"a1","event":{"started":{"kind":"Explore"}}}}"#,
    );
    written_as(
        &ReplyEvent::Agent {
            id: "a1".to_owned(),
            event: AgentEvent::Reported {
                text: "done".to_owned(),
                tokens: 9,
            },
        },
        r#"{"agent":{"id":"a1","event":{"reported":{"text":"done","tokens":9}}}}"#,
    );
    written_as(
        &ReplyEvent::Compacted {
            before: 10,
            after: 2,
        },
        r#"{"compacted":{"before":10,"after":2}}"#,
    );
}

#[test]
fn refuses_an_event_it_does_not_know() {
    assert!(from_str::<ReplyEvent>(r#"{"sung":"la"}"#).is_err());
}
