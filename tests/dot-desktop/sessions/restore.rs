use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use dot_api::{
    Asked, Carried, Cost, Detail, Ending, Failure, Hue, Opening, ReplyEvent, Text, ToolEnd,
};

use super::*;
use crate::app::tests::{IdTranslator, TestParts};
use crate::conversation::Recall;
use crate::event::Delivery;
use crate::keeping::tests::MemoryArchive;
use crate::reply::tests::{ScriptedChat, asked};
use crate::settings::tests::test_settings;
use crate::sign_in::FakeConnector;
use crate::status_bar::FixedSession;

type TestSessions = Sessions<TestParts>;

struct Window {
    sessions: TestSessions,
    archive: MemoryArchive,
    open: Opener<TestParts>,
    connector: Arc<FakeConnector>,
    sender: Sender<Delivery>,
    recalled: Arc<Mutex<Vec<Vec<u8>>>>,
}

fn window_in(folder: Option<&str>) -> Window {
    let (sender, _receiver) = mpsc::channel();
    let archive = MemoryArchive::default();
    let connector = Arc::new(FakeConnector::default());
    let recalled: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let made = Arc::clone(&recalled);
    let open: Opener<TestParts> = Box::new(move |_folder| {
        let chat = ScriptedChat {
            recalled: Arc::clone(&made),
            ..ScriptedChat::default()
        };
        (FixedSession::default(), chat)
    });
    let mut sessions = Sessions::new(
        (FixedSession::default(), ScriptedChat::default()),
        &connector,
        ((PermissionMode::Ask, test_settings().tuning), &sender),
        (Hue::Violet, 7),
        Arc::new(archive.clone()),
    );
    sessions.active_mut().folder = folder.map(PathBuf::from);
    Window {
        sessions,
        archive,
        open,
        connector,
        sender,
        recalled,
    }
}

impl Window {
    fn restore(&mut self, kept: Shelf) {
        let making = Making {
            open: &mut self.open,
            wiring: Wiring {
                connector: &self.connector,
                answering: (PermissionMode::Ask, test_settings().tuning),
                mailbox: &self.sender,
            },
        };
        self.sessions
            .restore(kept, making, (Instant::now(), &IdTranslator));
    }

    fn names(&self) -> Vec<Option<String>> {
        self.sessions
            .opened
            .iter()
            .map(|opened| opened.identity.name.clone())
            .collect()
    }
}

fn record(milliseconds: u64, happened: Happened<'static>) -> Record<'static> {
    Record {
        at: Duration::from_millis(milliseconds),
        happened,
    }
}

fn exchange(start: u64, text: &str) -> Vec<Record<'static>> {
    vec![
        record(
            start,
            Happened::Asked(Asked {
                text: Cow::Owned(text.to_owned()),
                opening: Opening::Prompt,
                images: Vec::new(),
            }),
        ),
        record(
            start + 10,
            Happened::Heard(Cow::Owned(ReplyEvent::Text("ok".to_owned()))),
        ),
        record(start + 20, Happened::Ended(Ending::default())),
    ]
}

fn kept(id: u64, name: &str, folder: Option<&str>) -> Restored {
    Restored {
        session: KeptSession {
            id: KeptId::new(id),
            name: Some(name.to_owned()),
            hue: Hue::Green,
            folder: folder.map(PathBuf::from),
        },
        records: exchange(0, name),
        memory: None,
    }
}

fn shelf(sessions: Vec<Restored>, shown: u64) -> Shelf {
    Shelf {
        sessions,
        shown: Some(KeptId::new(shown)),
    }
}

#[test]
fn an_empty_shelf_changes_nothing() {
    let mut window = window_in(None);
    let first = window.sessions.active().kept;

    window.restore(Shelf::default());

    assert_eq!(window.sessions.kept_ids(), [first]);
}

#[test]
fn the_kept_sessions_take_the_place_of_an_empty_session_without_folder() {
    let mut window = window_in(None);

    window.restore(shelf(vec![kept(1, "a", None), kept(2, "b", Some("/b"))], 2));

    assert_eq!(window.sessions.kept_ids(), [KeptId::new(1), KeptId::new(2)]);
    assert_eq!(window.sessions.active().kept, KeptId::new(2));
    assert_eq!(window.names(), [Some("a".to_owned()), Some("b".to_owned())]);
    let numbers: Vec<u64> = window
        .sessions
        .opened
        .iter()
        .map(|opened| opened.number)
        .collect();
    assert_eq!(numbers, [1, 2]);
}

#[test]
fn shows_the_last_kept_session_when_the_shown_one_is_unknown() {
    let mut window = window_in(None);

    window.restore(shelf(vec![kept(1, "a", None), kept(2, "b", None)], 9));

    assert_eq!(window.sessions.active().kept, KeptId::new(2));
}

#[test]
fn a_launch_in_the_folder_of_a_kept_session_shows_that_session() {
    let mut window = window_in(Some("/b"));

    window.restore(shelf(
        vec![kept(1, "a", Some("/a")), kept(2, "b", Some("/b"))],
        1,
    ));

    assert_eq!(window.sessions.kept_ids(), [KeptId::new(1), KeptId::new(2)]);
    assert_eq!(window.sessions.active().kept, KeptId::new(2));
}

#[test]
fn a_launch_in_another_folder_keeps_its_session_after_the_kept_ones() {
    let mut window = window_in(Some("/new"));
    let fresh = window.sessions.active().kept;

    window.restore(shelf(vec![kept(1, "a", Some("/a"))], 1));

    assert_eq!(window.sessions.kept_ids(), [KeptId::new(1), fresh]);
    assert_eq!(window.sessions.active().kept, fresh);
}

#[test]
fn a_session_already_in_use_is_never_replaced() {
    let mut window = window_in(None);
    let fresh = window.sessions.active().kept;
    window.sessions.active_mut().conversation.type_key(
        crate::conversation::Intent::Edit(crate::conversation::Edit::Insert("a".to_owned())),
        Instant::now(),
        || None,
        &IdTranslator,
    );

    window.restore(shelf(vec![kept(1, "a", None)], 1));

    assert_eq!(window.sessions.kept_ids(), [KeptId::new(1), fresh]);
    assert_eq!(window.sessions.active().kept, fresh);
}

#[test]
fn shows_each_kept_conversation_again() {
    let mut window = window_in(None);

    window.restore(shelf(vec![kept(1, "a", None)], 1));

    let restored = window.sessions.active();
    assert!(restored.conversation.has_exchanges());
    assert!(!restored.conversation.is_live());
    assert_eq!(restored.identity.hue, Hue::Green);
}

#[test]
fn counts_again_what_each_kept_session_spent() {
    let mut window = window_in(None);
    let mut records = vec![record(
        0,
        Happened::Carried(Carried {
            cost: Cost::from_micro_dollars(100),
            lines_added: 4,
            lines_removed: 1,
        }),
    )];
    records.extend(exchange(10, "a"));
    records.push(record(
        40,
        Happened::Heard(Cow::Owned(ReplyEvent::ToolEnded {
            id: "t".to_owned(),
            end: ToolEnd::Done(Detail::Changes {
                added: 3,
                removed: 2,
            }),
        })),
    ));
    records.push(record(
        50,
        Happened::Ended(Ending {
            spent: Reply {
                cost: Cost::from_micro_dollars(20),
                context_tokens: 900,
                ..Reply::default()
            },
            failure: None,
        }),
    ));
    let mut restored = kept(1, "a", None);
    restored.records = records;

    window.restore(shelf(vec![restored], 1));

    let usage = window.sessions.active().session.spending().usage;
    assert_eq!(usage.cost, Cost::from_micro_dollars(120));
    assert_eq!((usage.lines_added, usage.lines_removed), (7, 3));
    assert_eq!(usage.context_tokens, 900);
}

#[test]
fn a_fresh_start_or_a_crash_empties_the_context_again() {
    let spent = Happened::Ended(Ending {
        spent: Reply {
            context_tokens: 900,
            ..Reply::default()
        },
        failure: None,
    });
    let crashed = Happened::Ended(Ending {
        spent: Reply {
            context_tokens: 900,
            ..Reply::default()
        },
        failure: Some(Failure {
            text: Text::ReplyCrashed,
            cause: None,
        }),
    });
    let failed = Happened::Ended(Ending {
        spent: Reply {
            context_tokens: 900,
            ..Reply::default()
        },
        failure: Some(Failure {
            text: Text::ReplyOverloaded,
            cause: None,
        }),
    });
    for (last, emptied) in [
        (Happened::StartedOver, true),
        (crashed, true),
        (failed, false),
        (
            Happened::Asked(Asked {
                text: Cow::Borrowed("a"),
                opening: Opening::Prompt,
                images: Vec::new(),
            }),
            false,
        ),
    ] {
        let session = FixedSession::default();
        let records = [record(0, spent.clone()), record(10, last)];

        spend_again(&session, &records);

        assert_eq!(session.spending().usage.context_tokens == 0, emptied);
    }
}

#[test]
fn hands_each_chat_its_memory() {
    let mut window = window_in(None);
    let mut restored = kept(1, "a", None);
    restored.memory = Some(b"remembered".to_vec());

    window.restore(shelf(vec![restored, kept(2, "b", None)], 1));

    assert_eq!(*window.recalled.lock().unwrap(), [b"remembered".to_vec()]);
}

#[test]
fn a_kept_session_goes_on_in_its_own_journal_after_a_pause() {
    let mut window = window_in(None);

    window.restore(shelf(vec![kept(1, "a", None)], 1));
    window
        .sessions
        .active_mut()
        .ask(asked("Encore"), &mut Recall::default());
    crate::reply::tests::wait_until("the prompt is kept", || {
        window
            .archive
            .shelved()
            .journals
            .get(&KeptId::new(1))
            .is_some_and(|records| !records.is_empty())
    });

    let first = window.archive.shelved().journals[&KeptId::new(1)][0].at;
    assert_eq!(first, Duration::from_millis(20) + PAUSE_BETWEEN_LAUNCHES);
    window.sessions.stop_all();
}

#[test]
fn keeps_only_the_sessions_with_a_conversation_or_a_name() {
    let mut window = window_in(Some("/new"));
    let fresh = window.sessions.active().kept;

    assert_eq!(window.sessions.arrangement(), Arrangement::default());
    window.restore(shelf(vec![kept(1, "a", Some("/a"))], 1));
    let arrangement = window.sessions.arrangement();

    assert_eq!(arrangement.sessions.len(), 1);
    assert_eq!(arrangement.sessions[0].id, KeptId::new(1));
    assert_eq!(arrangement.sessions[0].folder, Some(PathBuf::from("/a")));
    assert_eq!(arrangement.shown, None);
    assert_eq!(window.sessions.active().kept, fresh);
    window.sessions.active_mut().identity.name = Some("named".to_owned());
    assert_eq!(window.sessions.arrangement().sessions.len(), 2);
    assert_eq!(window.sessions.arrangement().shown, Some(fresh));
}

#[test]
fn tells_whether_the_window_is_still_arranged_as_kept() {
    let mut window = window_in(None);
    window.restore(shelf(vec![kept(1, "a", None), kept(2, "b", None)], 1));
    let arrangement = window.sessions.arrangement();

    assert!(window.sessions.is_arranged_as(&arrangement));
    window.sessions.switch_to(1);
    assert!(!window.sessions.is_arranged_as(&arrangement));
    window.sessions.switch_to(0);
    window.sessions.paint(Hue::Pink);
    assert!(!window.sessions.is_arranged_as(&arrangement));
    window.sessions.paint(Hue::Green);
    assert!(window.sessions.is_arranged_as(&arrangement));
    window.sessions.active_mut().identity.name = None;
    assert!(!window.sessions.is_arranged_as(&arrangement));
    let shorter = Arrangement {
        sessions: arrangement.sessions[..1].to_vec(),
        shown: arrangement.shown,
    };
    assert!(!window.sessions.is_arranged_as(&shorter));
}

#[test]
fn shows_again_the_session_of_this_launch_that_was_shown() {
    let mut window = window_in(Some("/new"));
    let first = window.sessions.active().kept;
    window.sessions.open(
        (FixedSession::default(), ScriptedChat::default()),
        &window.connector,
        (
            (PermissionMode::Ask, test_settings().tuning),
            &window.sender,
        ),
    );
    let second = window.sessions.active().kept;
    window.sessions.active_mut().identity.name = Some("busy".to_owned());
    window.sessions.active_mut().conversation.type_key(
        crate::conversation::Intent::Edit(crate::conversation::Edit::Insert("a".to_owned())),
        Instant::now(),
        || None,
        &IdTranslator,
    );

    window.restore(shelf(vec![kept(1, "a", Some("/a"))], 1));

    assert_eq!(window.sessions.kept_ids(), [KeptId::new(1), first, second]);
    assert_eq!(window.sessions.active().kept, second);
}
