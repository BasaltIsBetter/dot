use std::borrow::Cow;

use dot_api::ApiKey;
use proptest::prelude::*;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

struct IdTranslator;

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }
}

fn row_text(line: &Line) -> String {
    line.iter().map(|span| span.text.as_str()).collect()
}

fn inks(line: &Line) -> Vec<Ink> {
    line.iter().map(|span| span.ink).collect()
}

fn draw(screen: &Screen) -> Vec<Line> {
    lines(screen, &IdTranslator)
}

#[test]
fn shows_nothing_while_checking_or_once_connected() {
    assert_eq!(draw(&Screen::Checking), Vec::<Line>::new());
    assert_eq!(draw(&Screen::Connected), Vec::<Line>::new());
}

#[test]
fn keeps_every_screen_on_the_same_rows() {
    let screens = [
        Screen::choosing(Method::ApiKey),
        Screen::WaitingForClaudeCode,
        Screen::EnteringKey {
            draft: String::new(),
            problem: None,
        },
    ];

    for screen in screens {
        let rows = draw(&screen);
        assert_eq!(rows.len(), ROWS);
        assert_eq!(row_text(&rows[TITLE_ROW]), "sign-in-title");
        assert_eq!(inks(&rows[TITLE_ROW]), [Ink::Full]);
    }
}

#[test]
fn puts_the_selected_way_in_full_ink_and_the_other_back() {
    let rows = draw(&Screen::choosing(Method::ApiKey));

    assert_eq!(
        row_text(&rows[3]),
        "  sign-in-claude-account   sign-in-claude-account-detail"
    );
    assert_eq!(inks(&rows[3]), [Ink::Receding, Ink::Faint]);
    assert_eq!(
        row_text(&rows[4]),
        "› sign-in-api-key          sign-in-api-key-detail"
    );
    assert_eq!(inks(&rows[4]), [Ink::Full, Ink::Secondary]);
    assert_eq!(row_text(&rows[HINT_ROW]), "sign-in-choose-hint");
}

#[test]
fn shows_the_problem_and_its_cause_below() {
    let screen = Screen::Choosing {
        selected: Method::ClaudeCode,
        problem: Some(Problem {
            text: Text::SignInClaudeCodeFailed,
            cause: Some("exit status: 1".to_owned()),
        }),
    };

    let rows = draw(&screen);

    assert_eq!(row_text(&rows[PROBLEM_ROW]), "sign-in-claude-code-failed");
    assert_eq!(inks(&rows[PROBLEM_ROW]), [Ink::Full]);
    assert_eq!(row_text(&rows[CAUSE_ROW]), "exit status: 1");
    assert_eq!(inks(&rows[CAUSE_ROW]), [Ink::Receding]);
}

#[test]
fn shows_claude_code_at_work_in_orange_with_a_way_out() {
    let rows = draw(&Screen::WaitingForClaudeCode);

    assert_eq!(inks(&rows[3]), [Ink::Full, Ink::Secondary]);
    assert_eq!(inks(&rows[4]), [Ink::Faint, Ink::Faint]);
    assert_eq!(row_text(&rows[HINT_ROW]), "sign-in-waiting-for-claude-code");
    assert_eq!(inks(&rows[HINT_ROW]), [Ink::Live]);
    assert_eq!(row_text(&rows[SECOND_HINT_ROW]), "sign-in-cancel-hint");
}

#[test]
fn hides_the_key_being_typed_behind_a_caret() {
    let screen = Screen::EnteringKey {
        draft: KEY.to_owned(),
        problem: None,
    };

    let rows = draw(&screen);

    assert_eq!(
        row_text(&rows[FIELD_ROW]),
        "› sk-ant-••••••••••••••••••cdef_"
    );
    assert_eq!(inks(&rows[FIELD_ROW]), [Ink::Full, Ink::Full]);
    assert_eq!(row_text(&rows[HINT_ROW]), "sign-in-key-hint");
}

#[test]
fn shows_the_key_being_checked_in_orange() {
    let screen = Screen::CheckingKey {
        key: ApiKey::parse(KEY).unwrap(),
    };

    let rows = draw(&screen);

    assert_eq!(inks(&rows[FIELD_ROW]), [Ink::Secondary]);
    assert_eq!(row_text(&rows[HINT_ROW]), "sign-in-checking-key");
    assert_eq!(inks(&rows[HINT_ROW]), [Ink::Live]);
}

#[test]
fn masks_a_short_or_unknown_draft_entirely() {
    assert_eq!(masked(""), "");
    assert_eq!(masked("sk-a"), "••••");
    assert_eq!(masked("sk-ant-"), "sk-ant-");
    assert_eq!(masked("sk-ant-abc"), "sk-ant-•••");
}

#[test]
fn caps_the_mask_of_a_long_key() {
    let long_key = format!("{KEY_PREFIX}{}wxyz", "a".repeat(100));

    assert_eq!(
        masked(&long_key),
        format!("{KEY_PREFIX}{}wxyz", "•".repeat(LONGEST_MASK))
    );
}

#[test]
fn finds_the_way_under_a_row_only_while_choosing() {
    let choosing = Screen::choosing(Method::ClaudeCode);

    assert_eq!(method_at(&choosing, 3), Some(Method::ClaudeCode));
    assert_eq!(method_at(&choosing, 4), Some(Method::ApiKey));
    assert_eq!(method_at(&choosing, 5), None);
    assert_eq!(method_at(&Screen::WaitingForClaudeCode, 3), None);
}

proptest! {
    #[test]
    fn never_shows_more_than_the_prefix_and_four_characters(draft in "[ -~]{0,300}") {
        let shown = masked(&draft);
        let visible: String = shown.chars().filter(|&character| character != MASK).collect();

        let tail = visible.strip_prefix(KEY_PREFIX).unwrap_or(&visible);

        prop_assert!(tail.len() <= SHOWN_TAIL);
        prop_assert!(draft.ends_with(tail));
    }
}
