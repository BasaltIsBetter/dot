use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use dot_api::{
    Arrangement, Asked, Connection, Ending, Happened, Hue, KeptId, KeptSession, Opening, Record,
    Restored, Shelf,
};
use winit::keyboard::NamedKey;

use super::*;
use crate::Keep;
use crate::app::DesktopApp;
use crate::app::tests::{
    IdTranslator, RETINA, RETINA_SIZE, RecordingCanvas, TestApp, act, choices_with, deliver,
    flow_texts, named, opener, typed, until_replied,
};
use crate::editor::tests::MemoryWorkspace;
use crate::event::{Delivery, WindowAction};
use crate::keeping::tests::{MemoryArchive, MemoryPrompts};
use crate::reply::tests::{ScriptedChat, connected, wait_until};
use crate::settings::tests::MemoryPreferences;
use crate::sign_in::Report;
use crate::status_bar::FixedSession;

const PATIENCE: Duration = Duration::from_secs(5);

fn kept_app(
    keep: Keep<MemoryArchive, MemoryPrompts>,
    chat: ScriptedChat,
) -> (TestApp, Receiver<Delivery>) {
    let (sender, receiver) = mpsc::channel();
    let mut app = DesktopApp::new(
        &IdTranslator,
        connected(),
        opener(chat),
        (choices_with(MemoryPreferences::default()), keep),
        (sender, Arc::new(MemoryWorkspace::default())),
    );
    app.canvas = Some(RecordingCanvas::default());
    app.window.size = RETINA_SIZE;
    app.window.scale_factor = RETINA;
    app.window.focused = true;
    deliver(
        &mut app,
        Delivery::SignIn(Report::Checked(Ok(Some(Connection::ApiKey)))),
        Instant::now(),
    );
    deliver(
        &mut app,
        Delivery::Session(FixedSession::default().0),
        Instant::now(),
    );
    (app, receiver)
}

fn land(app: &mut TestApp, receiver: &Receiver<Delivery>) {
    let mut landed = 0;
    while landed < 2 {
        let delivery = receiver.recv_timeout(PATIENCE).unwrap();
        if matches!(
            delivery,
            Delivery::Kept(KeptNews::Restored(_) | KeptNews::Prompts(_))
        ) {
            landed += 1;
        }
        deliver(app, delivery, Instant::now());
    }
}

fn kept(id: u64, name: &str) -> Restored {
    let asked = Record {
        at: Duration::ZERO,
        happened: Happened::Asked(Asked {
            text: Cow::Owned(format!("prompt of {name}")),
            opening: Opening::Prompt,
            images: Vec::new(),
        }),
    };
    let ended = Record {
        at: Duration::from_millis(10),
        happened: Happened::Ended(Ending::default()),
    };
    Restored {
        session: KeptSession {
            id: KeptId::new(id),
            name: Some(name.to_owned()),
            hue: Hue::Blue,
            folder: None,
        },
        records: vec![asked, ended],
        memory: None,
    }
}

#[test]
fn opens_the_kept_sessions_again_once_the_window_opens() {
    let archive = MemoryArchive::with(Shelf {
        sessions: vec![kept(1, "one"), kept(2, "two")],
        shown: Some(KeptId::new(1)),
    });
    let keep = Keep {
        archive: archive.clone(),
        prompts: MemoryPrompts::default(),
    };
    let (mut app, receiver) = kept_app(keep, ScriptedChat::default());
    let fresh = app.sessions.active().kept;

    app.start_keeping();
    land(&mut app, &receiver);

    assert_eq!(archive.shelved().alive, Some(vec![fresh]));
    assert_eq!(app.sessions.kept_ids(), [KeptId::new(1), KeptId::new(2)]);
    assert!(
        flow_texts(&app)
            .iter()
            .any(|text| text.contains("prompt of one")),
        "{:?}",
        flow_texts(&app)
    );
}

#[test]
fn keeps_the_arrangement_once_restored_and_each_time_it_changes() {
    let archive = MemoryArchive::with(Shelf {
        sessions: vec![kept(1, "one"), kept(2, "two")],
        shown: Some(KeptId::new(1)),
    });
    let keep = Keep {
        archive: archive.clone(),
        prompts: MemoryPrompts::default(),
    };
    let (mut app, receiver) = kept_app(keep, ScriptedChat::default());

    act(&mut app, WindowAction::Blur);
    act(&mut app, WindowAction::Focus);
    assert_eq!(archive.shelved().arranged, Vec::<Arrangement>::new());
    app.start_keeping();
    land(&mut app, &receiver);
    wait_until("the arrangement is kept", || {
        archive.shelved().arranged.len() == 1
    });
    act(
        &mut app,
        WindowAction::Key(crate::event::Keystroke {
            key: winit::keyboard::Key::Named(NamedKey::ArrowDown),
            text: None,
            modifiers: winit::keyboard::ModifiersState::SUPER,
        }),
    );
    wait_until("the shown session is kept", || {
        archive.shelved().arranged.len() == 2
    });

    let arranged = archive.shelved().arranged.clone();
    assert_eq!(arranged[0].shown, Some(KeptId::new(1)));
    assert_eq!(arranged[1].shown, Some(KeptId::new(2)));
    assert_eq!(arranged[1].sessions.len(), 2);
}

#[test]
fn up_writes_again_the_prompts_sent_and_those_kept() {
    let prompts = MemoryPrompts {
        loaded: vec!["older".to_owned()],
        ..MemoryPrompts::default()
    };
    let keep = Keep {
        archive: MemoryArchive::default(),
        prompts: prompts.clone(),
    };
    let chat = ScriptedChat::default();
    let sent = Arc::clone(&chat.prompts);
    let (mut app, receiver) = kept_app(keep, chat);
    app.start_keeping();
    land(&mut app, &receiver);

    act(&mut app, typed("Salut"));
    act(&mut app, named(NamedKey::Enter));
    until_replied(&mut app, &receiver);
    act(&mut app, named(NamedKey::ArrowUp));
    act(&mut app, named(NamedKey::ArrowUp));
    act(&mut app, named(NamedKey::Enter));
    until_replied(&mut app, &receiver);

    assert_eq!(*sent.lock().unwrap(), ["Salut", "older"]);
    wait_until("the prompts are kept", || {
        prompts
            .saved
            .lock()
            .unwrap()
            .last()
            .is_some_and(|saved| saved == &["Salut", "older"])
    });
}
