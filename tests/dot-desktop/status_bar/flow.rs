use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::{Cost, Limits, Model, Reply, Usage};
use winit::dpi::PhysicalSize;

use super::*;
use crate::status_bar::view::tests::{ShortTranslator, writing};

const PATIENCE: Duration = Duration::from_secs(5);

fn wide() -> Layout {
    Layout::reflowed(PhysicalSize::new(2880, 1800), 2.0)
}

fn narrow() -> Layout {
    Layout::reflowed(PhysicalSize::new(800, 1800), 2.0)
}

fn quiet() -> Activity {
    Activity::default()
}

pub(crate) struct FixedSession(pub(crate) SessionStatus, pub(crate) Mutex<Spending>);

impl Default for FixedSession {
    fn default() -> Self {
        Self(
            SessionStatus {
                model: Model::OPUS_5_5,
                effort: dot_api::Effort::High,
                project: Some("Dot".to_owned()),
                branch: Some("feat/mac-app".to_owned()),
                usage: Usage::default(),
                limits: Limits::default(),
            },
            Mutex::default(),
        )
    }
}

impl Session for FixedSession {
    fn status(&self) -> SessionStatus {
        self.0.clone()
    }

    fn spending(&self) -> Spending {
        *self.1.lock().unwrap()
    }

    fn record(&self, reply: &Reply) {
        let mut spending = self.1.lock().unwrap();
        let cost = spending.usage.cost.micro_dollars() + reply.cost.micro_dollars();
        spending.usage.cost = Cost::from_micro_dollars(cost);
        spending.usage.context_tokens = reply.context_tokens;
        spending.limits.five_hours = reply.limits.five_hours.or(spending.limits.five_hours);
        spending.limits.seven_days = reply.limits.seven_days.or(spending.limits.seven_days);
    }

    fn count_changes(&self, added: u64, removed: u64) {
        let mut spending = self.1.lock().unwrap();
        spending.usage.lines_added += added;
        spending.usage.lines_removed += removed;
    }

    fn clear_context(&self) {
        self.1.lock().unwrap().usage.context_tokens = 0;
    }
}

fn status_bar() -> (
    StatusBar<Sender<SessionStatus>>,
    Arc<FixedSession>,
    Receiver<SessionStatus>,
) {
    let (sender, receiver) = mpsc::channel();
    let session = Arc::new(FixedSession::default());
    (StatusBar::new(sender), session, receiver)
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
    let (mut status_bar, session, _receiver) = status_bar();

    assert_eq!(
        status_bar.strip(&ShortTranslator, &wide(), &quiet(), session.spending()),
        None
    );
}

#[test]
fn reads_the_session_in_the_background() {
    let (mut status_bar, session, receiver) = status_bar();

    status_bar.refresh(Arc::clone(&session));

    assert_eq!(
        receiver.recv_timeout(PATIENCE).unwrap(),
        FixedSession::default().0
    );
}

#[test]
fn arranges_the_latest_status_in_the_columns_given() {
    let (mut status_bar, session, _receiver) = status_bar();
    let mut status = FixedSession::default().0;
    status_bar.report(status.clone());
    status.branch = Some("main".to_owned());

    status_bar.report(status);

    let wide = status_bar
        .strip(&ShortTranslator, &wide(), &quiet(), session.spending())
        .unwrap();
    let narrow = status_bar
        .strip(&ShortTranslator, &narrow(), &quiet(), session.spending())
        .unwrap();
    assert!(text_of(&wide).trim_start().starts_with("Opus 5.5"));
    assert!(text_of(&wide).contains(" main "), "{}", text_of(&wide));
    assert!(!text_of(&narrow).contains("main"), "{}", text_of(&narrow));
    assert!(text_of(&narrow).trim_start().starts_with("Opus 5.5"));
}

#[test]
fn shows_the_middle_zone_as_soon_as_the_window_has_room_for_it() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    let without = Layout::reflowed(PhysicalSize::new(1024, 1800), 2.0);
    let with = Layout::reflowed(PhysicalSize::new(1025, 1800), 2.0);
    assert_eq!(without.strip_columns(), with.strip_columns());

    let first = status_bar
        .strip(&ShortTranslator, &without, &quiet(), session.spending())
        .unwrap();
    let second = status_bar
        .strip(&ShortTranslator, &with, &quiet(), session.spending())
        .unwrap();

    assert!(
        !text_of(&first).contains("feat/mac-app"),
        "{}",
        text_of(&first)
    );
    assert!(
        text_of(&second).contains("feat/mac-app"),
        "{}",
        text_of(&second)
    );
}

#[test]
fn shows_what_the_session_has_spent_so_far() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    let before = text_of(
        &status_bar
            .strip(&ShortTranslator, &wide(), &quiet(), session.spending())
            .unwrap(),
    );

    session.record(&Reply {
        context_tokens: 48_000,
        cost: Cost::from_micro_dollars(2_250_000),
        ..Reply::default()
    });
    session.count_changes(13, 1);

    let after = text_of(
        &status_bar
            .strip(&ShortTranslator, &wide(), &quiet(), session.spending())
            .unwrap(),
    );
    assert!(!before.contains("+13/−1"), "{before}");
    assert!(after.contains("+13/−1"), "{after}");
    assert!(after.contains("$2.25"), "{after}");
    assert!(after.contains("48k"), "{after}");
}

#[test]
fn keeps_a_limit_the_session_reported_until_a_reply_reports_one() {
    let (mut status_bar, session, _receiver) = status_bar();
    let mut status = FixedSession::default().0;
    status.limits.seven_days = Some(47);
    status_bar.report(status);
    let reported = text_of(
        &status_bar
            .strip(&ShortTranslator, &wide(), &quiet(), session.spending())
            .unwrap(),
    );

    session.record(&Reply {
        limits: Limits {
            five_hours: None,
            seven_days: Some(52),
        },
        ..Reply::default()
    });

    let replied = text_of(
        &status_bar
            .strip(&ShortTranslator, &wide(), &quiet(), session.spending())
            .unwrap(),
    );
    assert!(reported.contains("47"), "{reported}");
    assert!(replied.contains("52"), "{replied}");
}

#[test]
fn makes_the_same_strip_again_while_nothing_changes() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);

    let first = status_bar.strip(&ShortTranslator, &wide(), &quiet(), session.spending());
    let second = status_bar.strip(&ShortTranslator, &wide(), &quiet(), session.spending());

    assert_eq!(first, second);
    assert_eq!(
        status_bar.shown.as_ref().map(|shown| shown.columns),
        Some(wide().strip_columns())
    );
}

#[test]
fn makes_the_strip_again_when_what_claude_does_changes() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);

    let quiet = text_of(
        &status_bar
            .strip(&ShortTranslator, &wide(), &quiet(), session.spending())
            .unwrap(),
    );
    let busy = text_of(
        &status_bar
            .strip(&ShortTranslator, &wide(), &writing(), session.spending())
            .unwrap(),
    );

    assert!(!quiet.contains("Claude writes"), "{quiet}");
    assert!(busy.contains("Claude writes"), "{busy}");
}

#[test]
fn makes_the_strip_again_when_the_middle_zone_comes_or_goes() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    let wide = wide();
    let same_columns_without_middle = Layout::reflowed(PhysicalSize::new(1000, 1800), 2.0);
    assert!(!same_columns_without_middle.has_room_for_the_middle_zone());

    let with_middle = status_bar
        .strip(&ShortTranslator, &wide, &quiet(), session.spending())
        .unwrap();
    let without = status_bar
        .strip(
            &ShortTranslator,
            &same_columns_without_middle,
            &quiet(),
            session.spending(),
        )
        .unwrap();

    assert_eq!(with_middle.rules.len(), 2);
    assert_eq!(without.rules, Vec::<u16>::new());
}

#[test]
fn makes_the_strip_again_when_only_the_width_changes() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    let wider = Layout::reflowed(PhysicalSize::new(2_400, 1_200), 2.0);
    let widest = Layout::reflowed(PhysicalSize::new(2_880, 1_200), 2.0);

    let first = status_bar
        .strip(&ShortTranslator, &wider, &quiet(), session.spending())
        .unwrap();
    let second = status_bar
        .strip(&ShortTranslator, &widest, &quiet(), session.spending())
        .unwrap();

    assert_eq!(
        dot_transcript::columns(&text_of(&first)),
        usize::from(wider.strip_columns())
    );
    assert_eq!(
        dot_transcript::columns(&text_of(&second)),
        usize::from(widest.strip_columns())
    );
}
