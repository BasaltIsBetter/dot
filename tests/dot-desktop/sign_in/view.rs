use std::borrow::Cow;

use dot_api::{ApiKey, Connection};
use proptest::prelude::*;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

struct IdTranslator;

impl Translator for IdTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }
    fn count(&self, text: Text, _count: u64) -> Cow<'_, str> {
        self.translate(text)
    }
}

fn row_text(line: &Line) -> String {
    line.spans.iter().map(|span| span.text.as_str()).collect()
}

fn inks(line: &Line) -> Vec<Ink> {
    line.spans.iter().map(|span| span.ink).collect()
}

fn draw(screen: &Screen) -> Vec<Line> {
    lines(screen, &IdTranslator)
}

#[test]
fn shows_nothing_while_checking_or_once_connected() {
    assert_eq!(draw(&Screen::Checking), Vec::<Line>::new());
    assert_eq!(
        draw(&Screen::Connected(Connection::ApiKey)),
        Vec::<Line>::new()
    );
}

#[test]
fn keeps_every_screen_on_the_same_rows() {
    let screens = [
        Screen::choosing(Method::ApiKey),
        Screen::WaitingForSignIn(Account::ClaudeCode),
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
fn puts_the_selected_way_in_full_ink_and_the_others_back() {
    let rows = draw(&Screen::choosing(Method::ApiKey));
    let label = |text: &str| format!("{text:<28}");

    assert_eq!(
        row_text(&rows[3]),
        format!(
            "  {}sign-in-claude-account-detail",
            label("sign-in-claude-account")
        )
    );
    assert_eq!(inks(&rows[3]), [Ink::Receding, Ink::Faint]);
    assert_eq!(
        row_text(&rows[4]),
        format!(
            "  {}sign-in-anthropic-account-detail",
            label("sign-in-anthropic-account")
        )
    );
    assert_eq!(inks(&rows[4]), [Ink::Receding, Ink::Faint]);
    assert_eq!(
        row_text(&rows[5]),
        format!("› {}sign-in-api-key-detail", label("sign-in-api-key"))
    );
    assert_eq!(inks(&rows[5]), [Ink::Full, Ink::Secondary]);
    assert_eq!(row_text(&rows[HINT_ROW]), "sign-in-choose-hint");
}

#[test]
fn shows_the_account_sign_in_waiting_in_orange() {
    let rows = draw(&Screen::WaitingForSignIn(Account::Anthropic));

    assert!(row_text(&rows[4]).starts_with("› sign-in-anthropic-account"));
    assert_eq!(row_text(&rows[HINT_ROW]), "sign-in-waiting-for-ant");
    assert_eq!(inks(&rows[HINT_ROW]), [Ink::Live]);
    assert_eq!(row_text(&rows[SECOND_HINT_ROW]), "sign-in-cancel-hint");
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
    let rows = draw(&Screen::WaitingForSignIn(Account::ClaudeCode));

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

fn is_centred(line: &Line) -> bool {
    let width = u16::try_from(row_text(line).chars().count()).unwrap();
    line.indent == (Layout::GRID_COLUMNS - width) / 2
}

#[test]
fn centres_the_title_the_prompt_and_the_hint() {
    let rows = draw(&Screen::choosing(Method::ClaudeCode));

    for row in [TITLE_ROW, PROMPT_ROW, HINT_ROW] {
        assert!(is_centred(&rows[row]), "{:?}", rows[row]);
    }
}

#[test]
fn centres_both_ways_as_one_aligned_block() {
    let rows = draw(&Screen::choosing(Method::ApiKey));
    let widest = rows[3..=4]
        .iter()
        .map(|line| row_text(line).chars().count())
        .max()
        .unwrap();

    assert_eq!(rows[3].indent, rows[4].indent);
    assert_eq!(
        rows[3].indent,
        (Layout::GRID_COLUMNS - u16::try_from(widest).unwrap()) / 2
    );
}

#[test]
fn centres_the_problem_and_its_cause() {
    let screen = Screen::Choosing {
        selected: Method::ClaudeCode,
        problem: Some(Problem {
            text: Text::SignInClaudeCodeFailed,
            cause: Some("exit status: 1".to_owned()),
        }),
    };

    let rows = draw(&screen);

    assert!(is_centred(&rows[PROBLEM_ROW]));
    assert!(is_centred(&rows[CAUSE_ROW]));
}

#[test]
fn keeps_the_key_field_still_while_typing() {
    let indent_with = |draft: &str| {
        let screen = Screen::EnteringKey {
            draft: draft.to_owned(),
            problem: None,
        };
        draw(&screen)[FIELD_ROW].indent
    };

    assert_eq!(indent_with(""), indent_with(KEY));
    assert_eq!(indent_with(""), (Layout::GRID_COLUMNS - 46) / 2);
}

#[test]
fn never_indents_a_text_wider_than_the_column() {
    assert_eq!(indent_for(200), 0);
    assert_eq!(indent_for(64), 0);
    assert_eq!(indent_for(10), 27);
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
    assert_eq!(method_at(&choosing, 4), Some(Method::AnthropicAccount));
    assert_eq!(method_at(&choosing, 5), Some(Method::ApiKey));
    assert_eq!(method_at(&choosing, 6), None);
    assert_eq!(
        method_at(&Screen::WaitingForSignIn(Account::ClaudeCode), 3),
        None
    );
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
