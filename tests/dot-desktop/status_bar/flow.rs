use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::{Limits, Model, Usage};

use super::*;
use crate::status_bar::view::tests::ShortTranslator;

const PATIENCE: Duration = Duration::from_secs(5);
const WIDE: u16 = 120;

pub(crate) struct FixedSession(pub(crate) SessionStatus);

impl Default for FixedSession {
    fn default() -> Self {
        Self(SessionStatus {
            model: Model::OPUS_5_5,
            project: Some("Dot".to_owned()),
            branch: Some("feat/mac-app".to_owned()),
            usage: Usage::default(),
            limits: Limits::default(),
        })
    }
}

impl Session for FixedSession {
    fn status(&self) -> SessionStatus {
        self.0.clone()
    }
}

fn status_bar() -> (
    StatusBar<FixedSession, Sender<SessionStatus>>,
    Receiver<SessionStatus>,
) {
    let (sender, receiver) = mpsc::channel();
    (StatusBar::new(FixedSession::default(), sender), receiver)
}

fn text_of(strip: &Strip) -> String {
    strip
        .text
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

#[test]
fn shows_nothing_before_the_session_reports() {
    let (status_bar, _receiver) = status_bar();

    assert_eq!(status_bar.strip(&ShortTranslator, WIDE), None);
}

#[test]
fn reads_the_session_in_the_background() {
    let (mut status_bar, receiver) = status_bar();

    status_bar.refresh();

    assert_eq!(
        receiver.recv_timeout(PATIENCE).unwrap(),
        FixedSession::default().0
    );
}

#[test]
fn arranges_the_latest_status_in_the_columns_given() {
    let (mut status_bar, _receiver) = status_bar();
    let mut status = FixedSession::default().0;
    status_bar.report(status.clone());
    status.branch = Some("main".to_owned());

    status_bar.report(status);

    let wide = status_bar.strip(&ShortTranslator, WIDE).unwrap();
    let narrow = status_bar.strip(&ShortTranslator, 10).unwrap();
    assert!(text_of(&wide).trim_start().starts_with("Opus 5.5"));
    assert!(text_of(&wide).ends_with("main"));
    assert_eq!(text_of(&narrow).trim(), "Opus 5.5");
}
