use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use dot_api::{ConnectError, ConnectFailure, Connection};
use winit::keyboard::NamedKey;

use super::*;
use crate::app::tests::{act, connected_with_status, deliver, flow_texts, named, settled};
use crate::event::Delivery;
use crate::shell::ChosenFolder;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

fn command_comma() -> crate::event::WindowAction {
    crate::event::WindowAction::Key(crate::event::Keystroke {
        key: winit::keyboard::Key::Character(",".into()),
        text: Some(",".into()),
        modifiers: winit::keyboard::ModifiersState::SUPER,
    })
}

#[test]
fn names_the_folder_of_the_session_shown_or_none() {
    let mut app = connected_with_status();
    assert_eq!(app.actions().folder, Text::FolderNone.id());

    app.sessions.active_mut().folder = Some(PathBuf::from("/tmp/projet"));

    assert_eq!(app.actions().folder, "/tmp/projet");
}

#[test]
fn names_the_account_in_use() {
    let app = connected_with_status();

    assert_eq!(app.actions().account, Text::SignInApiKey.id());
}

#[test]
fn tells_the_key_kept_checked_or_typed() {
    let mut app = connected_with_status();
    assert_eq!(app.actions().key, Text::ApiKeyKept.id());

    app.try_key(ApiKey::parse(KEY).unwrap());
    assert_eq!(app.actions().key, Text::ApiKeyChecking.id());
    deliver(
        &mut app,
        Delivery::SignIn(Report::KeyReplaced(Ok(()))),
        Instant::now(),
    );

    assert_eq!(app.actions().key, "•••• cdef");
}

#[test]
fn says_why_a_key_was_refused_and_forgets_the_check() {
    let mut app = connected_with_status();
    app.try_key(ApiKey::parse(KEY).unwrap());

    deliver(
        &mut app,
        Delivery::SignIn(Report::KeyReplaced(Err(ConnectError::new(
            ConnectFailure::KeyRejected,
        )))),
        Instant::now(),
    );
    settled(&mut app);

    assert_eq!(app.actions().key, Text::ApiKeyKept.id());
    assert!(flow_texts(&app).contains(&Text::SignInKeyRejected.id().to_owned()));
}

#[test]
fn the_account_action_shows_the_choices_and_escape_comes_back() {
    let mut app = connected_with_status();
    act(&mut app, command_comma());

    app.act(Setting::Account, Instant::now());
    assert_eq!(app.screen, Screen::SignIn);
    assert!(!app.settings_screen.is_open());
    act(&mut app, named(NamedKey::Escape));

    assert_eq!(app.screen, Screen::Chat);
}

#[test]
fn signing_out_twice_confirmed_forgets_and_shows_the_choices() {
    let mut app = connected_with_status();
    let forgotten = Arc::clone(&app.connector.forgotten);
    act(&mut app, command_comma());
    for _ in 0..25 {
        act(&mut app, named(NamedKey::ArrowDown));
    }
    assert_eq!(app.settings_screen.focus(), Setting::SignOut);

    act(&mut app, named(NamedKey::Enter));
    assert!(!forgotten.load(Ordering::Relaxed));
    act(&mut app, named(NamedKey::Enter));
    crate::reply::tests::wait_until("Dot forgets its connection", || {
        forgotten.load(Ordering::Relaxed)
    });
    let report = Report::SignedOut(Ok(()));
    deliver(&mut app, Delivery::SignIn(report), Instant::now());

    assert_eq!(app.screen, Screen::SignIn);
    assert!(!app.settings_screen.is_open());
    assert_eq!(app.sign_in.connection(), None);
}

#[test]
fn a_failed_sign_out_says_why() {
    let mut app = connected_with_status();

    deliver(
        &mut app,
        Delivery::SignIn(Report::SignedOut(Err(ConnectError::new(
            ConnectFailure::StorageFailed,
        )))),
        Instant::now(),
    );
    settled(&mut app);

    assert_eq!(app.screen, Screen::Chat);
    assert!(flow_texts(&app).contains(&Text::SignInStorageFailed.id().to_owned()));
}

#[test]
fn a_chosen_folder_opens_a_new_session_there() {
    let mut app = connected_with_status();

    deliver(
        &mut app,
        Delivery::Folder(ChosenFolder(PathBuf::from("/tmp/ailleurs"))),
        Instant::now(),
    );

    assert_eq!(app.sessions.count(), "2/2");
    assert_eq!(
        app.sessions.active().folder,
        Some(PathBuf::from("/tmp/ailleurs"))
    );
}

#[test]
fn a_new_session_works_in_the_folder_of_the_one_shown() {
    let mut app = connected_with_status();
    app.sessions.active_mut().folder = Some(PathBuf::from("/tmp/ici"));

    app.open_session();

    assert_eq!(
        app.sessions.active().folder,
        Some(PathBuf::from("/tmp/ici"))
    );
}

#[test]
fn opens_a_file_of_another_folder_by_its_whole_path() {
    let mut app = connected_with_status();
    app.launch_folder = Some(PathBuf::from("/tmp/lance"));
    app.sessions.active_mut().folder = Some(PathBuf::from("/tmp/ailleurs"));

    assert_eq!(
        app.in_session_folder("src/a.rs".to_owned()),
        "/tmp/ailleurs/src/a.rs"
    );
    assert_eq!(app.in_session_folder("/etc/hosts".to_owned()), "/etc/hosts");
    app.sessions.active_mut().folder = Some(PathBuf::from("/tmp/lance"));
    assert_eq!(app.in_session_folder("src/a.rs".to_owned()), "src/a.rs");
    app.sessions.active_mut().folder = None;
    assert_eq!(app.in_session_folder("src/a.rs".to_owned()), "src/a.rs");
}

#[test]
fn shows_a_typed_key_by_its_last_four_characters() {
    assert_eq!(shown_key(&ApiKey::parse(KEY).unwrap()), "•••• cdef");
}

#[test]
fn a_new_connection_chosen_from_the_settings_returns_to_the_conversation() {
    let mut app = connected_with_status();
    app.act(Setting::Account, Instant::now());

    app.sign_in
        .input(crate::sign_in::Input::Press(Connection::AnthropicAccount));
    deliver(
        &mut app,
        Delivery::SignIn(Report::SignedIn(Ok(()))),
        Instant::now(),
    );

    assert_eq!(app.screen, Screen::Chat);
    assert_eq!(app.sign_in.connection(), Some(Connection::AnthropicAccount));
}
