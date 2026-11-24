use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use dot_api::{Cost, Limits, Reply, Usage};
use winit::dpi::PhysicalSize;

use super::*;
use crate::status_bar::view::tests::{OPUS, ShortTranslator, writing};

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
        status_bar.strip(
            &ShortTranslator,
            &wide(),
            (&quiet(), &OPUS),
            session.spending()
        ),
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
        .strip(
            &ShortTranslator,
            &wide(),
            (&quiet(), &OPUS),
            session.spending(),
        )
        .unwrap();
    let narrow = status_bar
        .strip(
            &ShortTranslator,
            &narrow(),
            (&quiet(), &OPUS),
            session.spending(),
        )
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
        .strip(
            &ShortTranslator,
            &without,
            (&quiet(), &OPUS),
            session.spending(),
        )
        .unwrap();
    let second = status_bar
        .strip(
            &ShortTranslator,
            &with,
            (&quiet(), &OPUS),
            session.spending(),
        )
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
            .strip(
                &ShortTranslator,
                &wide(),
                (&quiet(), &OPUS),
                session.spending(),
            )
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
            .strip(
                &ShortTranslator,
                &wide(),
                (&quiet(), &OPUS),
                session.spending(),
            )
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
            .strip(
                &ShortTranslator,
                &wide(),
                (&quiet(), &OPUS),
                session.spending(),
            )
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
            .strip(
                &ShortTranslator,
                &wide(),
                (&quiet(), &OPUS),
                session.spending(),
            )
            .unwrap(),
    );
    assert!(reported.contains("47"), "{reported}");
    assert!(replied.contains("52"), "{replied}");
}

#[test]
fn makes_the_same_strip_again_while_nothing_changes() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);

    let first = status_bar.strip(
        &ShortTranslator,
        &wide(),
        (&quiet(), &OPUS),
        session.spending(),
    );
    let second = status_bar.strip(
        &ShortTranslator,
        &wide(),
        (&quiet(), &OPUS),
        session.spending(),
    );

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
            .strip(
                &ShortTranslator,
                &wide(),
                (&quiet(), &OPUS),
                session.spending(),
            )
            .unwrap(),
    );
    let busy = text_of(
        &status_bar
            .strip(
                &ShortTranslator,
                &wide(),
                (&writing(), &OPUS),
                session.spending(),
            )
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
        .strip(
            &ShortTranslator,
            &wide,
            (&quiet(), &OPUS),
            session.spending(),
        )
        .unwrap();
    let without = status_bar
        .strip(
            &ShortTranslator,
            &same_columns_without_middle,
            (&quiet(), &OPUS),
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
        .strip(
            &ShortTranslator,
            &wider,
            (&quiet(), &OPUS),
            session.spending(),
        )
        .unwrap();
    let second = status_bar
        .strip(
            &ShortTranslator,
            &widest,
            (&quiet(), &OPUS),
            session.spending(),
        )
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

#[test]
fn arranges_the_strip_again_when_the_tuning_changes() {
    let (mut status_bar, session, _receiver) = status_bar();
    status_bar.report(FixedSession::default().0);
    let haiku = dot_api::Tuning {
        model: dot_api::Model::HAIKU_5_5,
        ..OPUS
    };

    let first = status_bar
        .strip(
            &ShortTranslator,
            &wide(),
            (&quiet(), &OPUS),
            session.spending(),
        )
        .unwrap();
    let second = status_bar
        .strip(
            &ShortTranslator,
            &wide(),
            (&quiet(), &haiku),
            session.spending(),
        )
        .unwrap();

    assert!(text_of(&first).trim_start().starts_with("Opus 5.5"));
    assert!(text_of(&second).trim_start().starts_with("Haiku 5.5"));
}

#[test]
fn arranges_the_bar_of_the_settings_in_the_columns_of_the_window() {
    let searching = Searching {
        query: "ef".to_owned(),
        matching: 1,
        saved_heat: 0,
    };

    let strip =
        StatusBar::<Sender<SessionStatus>>::searching(&ShortTranslator, &wide(), &searching);

    let text = text_of(&strip);
    assert!(text.starts_with("◇ settings-title ⌕ ef_ · 1"), "{text}");
    assert!(text.ends_with("◇"), "{text}");
    assert_eq!(
        dot_transcript::columns(&text),
        usize::from(wide().strip_columns())
    );
    assert!(text.contains(dot_api::Text::SettingsSaved.id()));
    let narrow =
        StatusBar::<Sender<SessionStatus>>::searching(&ShortTranslator, &narrow(), &searching);
    assert!(!text_of(&narrow).contains(dot_api::Text::SettingsSaved.id()));
}

const VIOLET: [u8; 3] = [0xa9, 0x70, 0xff];

#[test]
fn colours_the_gauge_once_the_context_is_four_fifths_full() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();

    assert_eq!(status_bar.gauge_tint((799, 1_000), Some(VIOLET), now), None);
    assert_eq!(
        status_bar.gauge_tint((800, 1_000), Some(VIOLET), now),
        Some(VIOLET)
    );
}

#[test]
fn warms_the_gauge_when_the_context_grows_then_cools_it_in_a_second_and_a_fifth() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_tint((100, 1_000), Some(VIOLET), now);

    let grown = status_bar.gauge_tint((200, 1_000), Some(VIOLET), now);
    let later = now + Duration::from_millis(600);
    let cooling = status_bar.gauge_tint((200, 1_000), Some(VIOLET), later);

    assert_eq!(grown, Some(VIOLET));
    assert!(cooling.is_some_and(|tint| tint != VIOLET && tint != [255; 3]));
    assert!(status_bar.is_cooling(later));
    let cooled = now + Duration::from_millis(1_200);
    assert_eq!(
        status_bar.gauge_tint((200, 1_000), Some(VIOLET), cooled),
        None
    );
    assert!(!status_bar.is_cooling(cooled));
}

#[test]
fn never_warms_a_gauge_that_did_not_grow() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_tint((300, 1_000), Some(VIOLET), now);

    assert_eq!(status_bar.gauge_tint((300, 1_000), Some(VIOLET), now), None);
    assert_eq!(status_bar.gauge_tint((100, 1_000), Some(VIOLET), now), None);
}

#[test]
fn never_colours_the_gauge_without_state_colours() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_tint((100, 1_000), None, now);

    assert_eq!(status_bar.gauge_tint((900, 1_000), None, now), None);
}

#[test]
fn a_new_session_s_status_never_counts_as_growth() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_tint((100, 1_000), Some(VIOLET), now);

    status_bar.report(FixedSession::default().0);

    assert_eq!(status_bar.gauge_tint((500, 1_000), Some(VIOLET), now), None);
    assert!(!status_bar.is_cooling(now));
}

#[test]
fn another_session_s_context_never_counts_as_growth() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_for(SessionKey(1));
    status_bar.gauge_tint((100, 1_000), Some(VIOLET), now);

    status_bar.gauge_for(SessionKey(2));

    assert_eq!(status_bar.gauge_tint((500, 1_000), Some(VIOLET), now), None);
    assert!(!status_bar.is_cooling(now));
}

#[test]
fn the_same_session_still_warms_its_gauge() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_for(SessionKey(1));
    status_bar.gauge_tint((100, 1_000), Some(VIOLET), now);

    status_bar.gauge_for(SessionKey(1));

    assert_eq!(
        status_bar.gauge_tint((500, 1_000), Some(VIOLET), now),
        Some(VIOLET)
    );
}

#[test]
fn a_window_of_zero_tokens_never_divides_by_zero() {
    let (mut status_bar, _session, _receiver) = status_bar();

    assert_eq!(
        status_bar.gauge_tint((5, 0), Some(VIOLET), Instant::now()),
        Some(VIOLET)
    );
}

#[test]
fn glides_the_gauge_to_its_new_share_in_three_hundred_milliseconds() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    assert_eq!(status_bar.gauge_fill((100, 1_000), now), 100);

    let start = status_bar.gauge_fill((500, 1_000), now);
    let halfway = status_bar.gauge_fill((500, 1_000), now + Duration::from_millis(100));

    assert_eq!(start, 100);
    assert!(halfway > 100 && halfway < 500, "{halfway}");
    assert!(status_bar.is_cooling(now + Duration::from_millis(100)));
    let settled = now + Duration::from_millis(300);
    assert_eq!(status_bar.gauge_fill((500, 1_000), settled), 500);
}

#[test]
fn shows_a_new_session_s_gauge_at_once_and_never_past_full() {
    let (mut status_bar, _session, _receiver) = status_bar();
    let now = Instant::now();
    status_bar.gauge_fill((100, 1_000), now);

    status_bar.report(FixedSession::default().0);

    assert_eq!(status_bar.gauge_fill((700, 1_000), now), 700);
    status_bar.gauge_fill((5_000, 1_000), now + Duration::from_secs(1));
    assert_eq!(
        status_bar.gauge_fill((5_000, 1_000), now + Duration::from_secs(2)),
        1_000
    );
    assert_eq!(whole(-3.0), 0);
    assert_eq!(whole(2_000.0), 1_000);
}
