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
            effort: dot_api::Effort::High,
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
    let narrow = status_bar.strip(&ShortTranslator, 11).unwrap();
    assert!(text_of(&wide).trim_start().starts_with("Opus 5.5"));
    assert!(text_of(&wide).ends_with("main"));
    assert_eq!(text_of(&narrow).trim(), "Opus 5.5");
}

#[test]
fn adds_up_the_cost_of_replies_and_keeps_the_last_context() {
    let (mut status_bar, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    let reply = |context_tokens, micro_dollars| Reply {
        context_tokens,
        cost: Cost::from_micro_dollars(micro_dollars),
        limits: Limits::default(),
    };

    status_bar.record(&reply(12_000, 1_250_000));
    status_bar.record(&reply(48_000, 1_000_000));

    let status = status_bar.status.as_ref().unwrap();
    assert_eq!(status.usage.context_tokens, 48_000);
    assert_eq!(status.usage.cost, Cost::from_micro_dollars(2_250_000));
}

#[test]
fn keeps_what_replies_consumed_when_the_session_reports_again() {
    let (mut status_bar, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    status_bar.record(&Reply {
        context_tokens: 9_000,
        cost: Cost::from_micro_dollars(10),
        limits: Limits {
            five_hours: None,
            seven_days: Some(47),
        },
    });

    status_bar.report(FixedSession::default().0);

    let status = status_bar.status.as_ref().unwrap();
    assert_eq!(status.usage.context_tokens, 9_000);
    assert_eq!(status.usage.cost, Cost::from_micro_dollars(10));
    assert_eq!(status.limits.seven_days, Some(47));
}

#[test]
fn keeps_a_limit_a_later_reply_does_not_report() {
    let (mut status_bar, _receiver) = status_bar();
    let limited = Reply {
        limits: Limits {
            five_hours: Some(3),
            seven_days: Some(47),
        },
        ..Reply::default()
    };
    status_bar.record(&limited);
    status_bar.record(&Reply::default());

    status_bar.report(FixedSession::default().0);

    let status = status_bar.status.as_ref().unwrap();
    assert_eq!(status.limits.five_hours, Some(3));
    assert_eq!(status.limits.seven_days, Some(47));
}

#[test]
fn adds_up_the_lines_tools_changed_and_keeps_them_when_the_session_reports_again() {
    let (mut status_bar, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);

    status_bar.count_changes(3, 1);
    status_bar.count_changes(10, 0);
    status_bar.report(FixedSession::default().0);

    let usage = status_bar.status.as_ref().unwrap().usage;
    assert_eq!((usage.lines_added, usage.lines_removed), (13, 1));
    let strip = text_of(&status_bar.strip(&ShortTranslator, WIDE).unwrap());
    assert!(strip.contains("+13 −1"), "{strip}");
}
