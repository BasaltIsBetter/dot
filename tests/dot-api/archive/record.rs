use serde_json::{from_str, to_string};

use super::*;
use crate::session::Limits;
use crate::tool::{Detail, ToolEnd};

fn written_as(happened: &Happened<'_>, written: &str) {
    assert_eq!(to_string(happened).expect("it serializes"), written);
    assert_eq!(
        &from_str::<Happened<'_>>(written).expect("it reads back"),
        happened
    );
}

fn ended(added: u64, removed: u64) -> ReplyEvent {
    ReplyEvent::ToolEnded {
        id: "t".to_owned(),
        end: ToolEnd::Done(Detail::Changes { added, removed }),
    }
}

fn ending(micro_dollars: u64) -> Happened<'static> {
    Happened::Ended(Ending {
        spent: Reply {
            cost: Cost::from_micro_dollars(micro_dollars),
            ..Reply::default()
        },
        failure: None,
    })
}

#[test]
fn keeps_what_was_asked_under_stable_names() {
    written_as(
        &Happened::Asked(Asked {
            text: Cow::Borrowed("Fix it"),
            opening: Opening::Prompt,
            images: Vec::new(),
        }),
        r#"{"asked":{"text":"Fix it","opening":"prompt"}}"#,
    );
    written_as(
        &Happened::Asked(Asked {
            text: Cow::Borrowed(""),
            opening: Opening::Redirect,
            images: vec![ImageLabel {
                name: "shot.png".to_owned(),
                bytes: 1_024,
            }],
        }),
        r#"{"asked":{"text":"","opening":"redirect","images":[{"name":"shot.png","bytes":1024}]}}"#,
    );
    written_as(
        &Happened::Asked(Asked {
            text: Cow::Borrowed("so"),
            opening: Opening::Note,
            images: Vec::new(),
        }),
        r#"{"asked":{"text":"so","opening":"note"}}"#,
    );
}

#[test]
fn keeps_what_was_heard_under_the_names_of_the_event() {
    written_as(
        &Happened::Heard(Cow::Owned(ReplyEvent::Text("Hi".to_owned()))),
        r#"{"heard":{"text":"Hi"}}"#,
    );
}

#[test]
fn keeps_endings_under_stable_names() {
    written_as(
        &Happened::Ended(Ending {
            spent: Reply {
                context_tokens: 1,
                base_tokens: None,
                output_tokens: 2,
                cost: Cost::from_micro_dollars(3),
                limits: Limits::default(),
            },
            failure: Some(Failure {
                text: Text::ReplyOverloaded,
                cause: Some("HTTP 529".to_owned()),
            }),
        }),
        concat!(
            r#"{"ended":{"spent":{"context-tokens":1,"base-tokens":null,"output-tokens":2,"cost":3,"#,
            r#""limits":{"five-hours":null,"seven-days":null}},"#,
            r#""failure":{"text":"reply-overloaded","cause":"HTTP 529"}}}"#
        ),
    );
    written_as(
        &ending(0),
        r#"{"ended":{"spent":{"context-tokens":0,"base-tokens":null,"output-tokens":0,"cost":0,"limits":{"five-hours":null,"seven-days":null}}}}"#,
    );
}

#[test]
fn keeps_fresh_starts_and_carried_spending_under_stable_names() {
    written_as(&Happened::StartedOver, r#""started-over""#);
    written_as(
        &Happened::Carried(Carried {
            cost: Cost::from_micro_dollars(5),
            lines_added: 6,
            lines_removed: 7,
        }),
        r#"{"carried":{"cost":5,"lines-added":6,"lines-removed":7}}"#,
    );
}

#[test]
fn reads_an_ending_with_missing_parts_as_a_reply_that_spent_nothing() {
    assert_eq!(
        from_str::<Happened<'_>>(r#"{"ended":{}}"#).expect("missing parts default"),
        Happened::Ended(Ending::default())
    );
}

#[test]
fn refuses_what_it_does_not_know() {
    for written in [
        r#"{"sung":{}}"#,
        r#"{"asked":{"text":"a","opening":"shouted"}}"#,
        r#"{"ended":{"failure":{"text":"no-such-text"}}}"#,
        r#""asked""#,
    ] {
        assert!(from_str::<Happened<'_>>(written).is_err(), "{written}");
    }
}

#[test]
fn owns_what_it_borrowed() {
    let text = String::from("Fix it");
    let event = ReplyEvent::Text("Hi".to_owned());
    let asked = Happened::Asked(Asked {
        text: Cow::Borrowed(&text),
        opening: Opening::Note,
        images: Vec::new(),
    });
    let heard = Happened::Heard(Cow::Borrowed(&event));

    let owned = [asked.clone().into_owned(), heard.clone().into_owned()];

    assert_eq!(owned, [asked, heard]);
    assert!(matches!(
        &owned[0],
        Happened::Asked(Asked {
            text: Cow::Owned(_),
            ..
        })
    ));
    assert!(matches!(&owned[1], Happened::Heard(Cow::Owned(_))));
    for other in [
        ending(1),
        Happened::StartedOver,
        Happened::Carried(Carried::default()),
    ] {
        assert_eq!(other.clone().into_owned(), other);
    }
}

#[test]
fn an_exchange_opens_with_what_was_asked_or_a_fresh_start() {
    let asked = Happened::Asked(Asked {
        text: Cow::Borrowed("a"),
        opening: Opening::Prompt,
        images: Vec::new(),
    });

    assert!(asked.opens_an_exchange());
    assert!(Happened::StartedOver.opens_an_exchange());
    assert!(!ending(0).opens_an_exchange());
    assert!(!Happened::Heard(Cow::Owned(ReplyEvent::Text("a".to_owned()))).opens_an_exchange());
    assert!(!Happened::Carried(Carried::default()).opens_an_exchange());
}

#[test]
fn carries_costs_and_changed_lines() {
    let mut carried = Carried::default();

    carried.add(&ending(3));
    carried.add(&ending(4));
    carried.add(&Happened::Heard(Cow::Owned(ended(10, 2))));
    carried.add(&Happened::Heard(Cow::Owned(ReplyEvent::Text(
        "a".to_owned(),
    ))));
    carried.add(&Happened::StartedOver);
    carried.add(&Happened::Carried(Carried {
        cost: Cost::from_micro_dollars(100),
        lines_added: 1,
        lines_removed: 1,
    }));

    assert_eq!(
        carried,
        Carried {
            cost: Cost::from_micro_dollars(107),
            lines_added: 11,
            lines_removed: 3,
        }
    );
}

#[test]
fn carries_without_overflowing() {
    let mut carried = Carried {
        cost: Cost::from_micro_dollars(u64::MAX),
        lines_added: u64::MAX,
        lines_removed: u64::MAX,
    };

    carried.add(&ending(1));
    carried.add(&Happened::Heard(Cow::Owned(ended(1, 1))));
    carried.add(&Happened::Carried(carried));

    assert_eq!(carried.cost.micro_dollars(), u64::MAX);
    assert_eq!(carried.lines_added, u64::MAX);
    assert_eq!(carried.lines_removed, u64::MAX);
}

#[test]
fn tells_the_lines_a_tool_changed() {
    assert_eq!(changed_lines(&ended(3, 1)), Some((3, 1)));
    assert_eq!(
        changed_lines(&ReplyEvent::ToolEnded {
            id: "t".to_owned(),
            end: ToolEnd::Done(Detail::Lines(3)),
        }),
        None
    );
    assert_eq!(
        changed_lines(&ReplyEvent::ToolEnded {
            id: "t".to_owned(),
            end: ToolEnd::Failed,
        }),
        None
    );
    assert_eq!(changed_lines(&ReplyEvent::Text("3".to_owned())), None);
}
