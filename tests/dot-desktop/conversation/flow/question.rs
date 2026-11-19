use dot_api::Action;

use super::*;
use crate::canvas::Layout;
use crate::conversation::flow::tests::{IdTranslator, compact, layout};

const ROW: f32 = 20.0;
const BAND: Band = Band {
    top: 100.0,
    bottom: 400.0,
};

fn permission(action: Action) -> Permission {
    Permission {
        tool_id: "t1".to_owned(),
        action,
    }
}

fn run(command: &str) -> Permission {
    permission(Action::Run {
        command: command.to_owned(),
    })
}

fn question(permission: &Permission, now: Instant) -> Question {
    Question::new(permission, TextWidth::of(&layout()), now, &IdTranslator)
}

fn later(now: Instant) -> Instant {
    now + ANSWER_GUARD
}

fn texts(question: &Question, shown: usize) -> Vec<String> {
    question
        .lines(BAND, shown, ROW)
        .iter()
        .map(|placed| {
            placed
                .line
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect()
        })
        .collect()
}

fn plain(key: NamedKey) -> Key {
    Key::Named(key)
}

#[test]
fn asks_shows_the_subject_then_the_keys() {
    let now = Instant::now();
    let question = question(&run("cargo test"), now);

    let lines = question.lines(BAND, 3, ROW);

    assert_eq!(question.rows(), 3);
    assert_eq!(
        texts(&question, 3),
        vec![
            "› permission-run",
            "cargo test",
            "permission-allow   permission-deny   grant-command",
        ]
    );
    let inks: Vec<Vec<Ink>> = lines
        .iter()
        .map(|placed| placed.line.spans.iter().map(|span| span.ink).collect())
        .collect();
    assert_eq!(
        inks,
        vec![
            vec![Ink::Live, Ink::Secondary],
            vec![Ink::Full],
            vec![Ink::Receding]
        ]
    );
    let indents: Vec<u16> = lines.iter().map(|placed| placed.line.indent).collect();
    assert_eq!(indents, [0, TEXT_INDENT, TEXT_INDENT]);
    let tops: Vec<f32> = lines.iter().map(|placed| placed.top).collect();
    assert_eq!(tops, [100.0, 120.0, 140.0]);
}

#[test]
fn names_what_yes_always_covers_for_each_action() {
    let now = Instant::now();
    let change = permission(Action::Change {
        path: "src/lib.rs".to_owned(),
    });
    let fetch = permission(Action::Fetch {
        url: "https://docs.rs/serde".to_owned(),
    });

    assert_eq!(
        texts(&question(&change, now), 3),
        vec![
            "› permission-change",
            "src/lib.rs",
            "permission-allow   permission-deny   grant-changes",
        ]
    );
    assert_eq!(
        texts(&question(&fetch, now), 3)[2].split("   ").last(),
        Some("grant-host")
    );
}

#[test]
fn shows_a_long_command_whole_over_several_rows() {
    let now = Instant::now();
    let command = format!("echo start\n{}\necho end", "x".repeat(150));
    let question = question(&run(&command), now);

    let shown = texts(&question, question.rows());

    assert_eq!(question.rows(), 2 + 4);
    assert_eq!(shown[1], "echo start");
    assert_eq!(shown[2].chars().count() + shown[3].chars().count(), 150);
    assert_eq!(shown[4], "echo end");
}

#[test]
fn reveals_characters_that_would_hide_part_of_the_command() {
    let now = Instant::now();
    let question = question(&run("ls\u{202e}fr- mr"), now);

    assert_eq!(texts(&question, 3)[1], "ls\u{fffd}fr- mr");
}

#[test]
fn starts_at_the_question_and_scrolls_down_to_the_keys() {
    let now = Instant::now();
    let lines: Vec<String> = (0..20).map(|line| format!("echo {line}")).collect();
    let mut question = question(&run(&lines.join("\n")), now);

    let first = texts(&question, 5);
    question.scroll_by(-100.0, 5);
    let last = texts(&question, 5);

    assert_eq!(first[0], "› permission-run");
    assert_eq!(first.len(), 5);
    assert_eq!(
        last[4],
        "permission-allow   permission-deny   grant-command"
    );
    assert_eq!(last[3], "echo 19");
    let tops: Vec<f32> = question
        .lines(BAND, 5, ROW)
        .iter()
        .map(|placed| placed.top)
        .collect();
    assert_eq!(tops, [100.0, 120.0, 140.0, 160.0, 180.0]);
}

#[test]
fn rewraps_the_subject_only_when_the_width_changes() {
    let now = Instant::now();
    let mut question = question(&run(&"word ".repeat(30)), now);
    let wide = question.rows();

    question.rewrap_to(TextWidth::of(&layout()));
    assert_eq!(question.rows(), wide);

    question.rewrap_to(TextWidth::of(&compact()));
    assert!(question.rows() > wide);
}

#[test]
fn cuts_the_question_and_the_keys_to_a_narrow_column() {
    let now = Instant::now();
    let narrow = Layout::reflowed(winit::dpi::PhysicalSize::new(400, 600), 1.0);
    let width = TextWidth::of(&narrow);
    let question = Question::new(&run("ls"), width, now, &IdTranslator);

    let shown = texts(&question, 3);

    assert_eq!(shown[2].chars().count(), width.columns());
    assert!(shown[2].ends_with('…'));
}

#[test]
fn answers_with_return_tab_or_escape_once_the_guard_is_over() {
    let now = Instant::now();
    let question = question(&run("ls"), now);
    let none = ModifiersState::empty();

    assert_eq!(question.answer_to(&plain(NamedKey::Enter), none, now), None);
    let just_before = now + ANSWER_GUARD.saturating_sub(Duration::from_millis(1));
    assert_eq!(
        question.answer_to(&plain(NamedKey::Escape), none, just_before),
        None
    );

    let ready = later(now);
    assert_eq!(
        question.answer_to(&plain(NamedKey::Enter), none, ready),
        Some(Answer::Allow)
    );
    assert_eq!(
        question.answer_to(&plain(NamedKey::Tab), none, ready),
        Some(Answer::Always)
    );
    assert_eq!(
        question.answer_to(&plain(NamedKey::Escape), none, ready),
        Some(Answer::Deny)
    );
}

#[test]
fn ignores_other_keys_and_keys_with_modifiers() {
    let now = Instant::now();
    let question = question(&run("ls"), now);
    let ready = later(now);

    let ignored = [
        (plain(NamedKey::Enter), ModifiersState::SHIFT),
        (plain(NamedKey::Tab), ModifiersState::SHIFT),
        (plain(NamedKey::Enter), ModifiersState::SUPER),
        (plain(NamedKey::Enter), ModifiersState::CONTROL),
        (plain(NamedKey::Escape), ModifiersState::ALT),
        (plain(NamedKey::Space), ModifiersState::empty()),
        (Key::Character("y".into()), ModifiersState::empty()),
    ];

    for (key, modifiers) in ignored {
        assert_eq!(question.answer_to(&key, modifiers, ready), None, "{key:?}");
    }
    assert_eq!(
        question.answer_to(&plain(NamedKey::Escape), ModifiersState::SHIFT, ready),
        Some(Answer::Deny)
    );
}
