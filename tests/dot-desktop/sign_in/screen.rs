use std::io;

use dot_api::ConnectFailure;
use proptest::prelude::*;

use super::*;

const KEY: &str = "sk-ant-api03-0123456789abcdef";

fn entering(draft: &str) -> Screen {
    Screen::EnteringKey {
        draft: draft.to_owned(),
        problem: None,
    }
}

fn checking_key() -> Screen {
    Screen::CheckingKey {
        key: ApiKey::parse(KEY).unwrap(),
    }
}

fn failure(failure: ConnectFailure) -> ConnectError {
    ConnectError::new(failure)
}

fn type_text(text: &str) -> Input {
    Input::Type(text.to_owned())
}

#[test]
fn arrows_move_the_selection_through_the_three_ways_and_around() {
    let mut screen = Screen::choosing(Method::ClaudeCode);

    assert_eq!(screen.input(Input::Next), Command::Nothing);
    assert_eq!(screen, Screen::choosing(Method::AnthropicAccount));
    screen.input(Input::Next);
    assert_eq!(screen, Screen::choosing(Method::ApiKey));
    screen.input(Input::Next);
    assert_eq!(screen, Screen::choosing(Method::ClaudeCode));
    screen.input(Input::Previous);
    assert_eq!(screen, Screen::choosing(Method::ApiKey));
    screen.input(Input::Previous);
    assert_eq!(screen, Screen::choosing(Method::AnthropicAccount));
}

#[test]
fn confirming_the_anthropic_account_signs_in_through_ant() {
    let mut screen = Screen::choosing(Method::AnthropicAccount);

    assert_eq!(
        screen.input(Input::Confirm),
        Command::SignIn(Account::Anthropic)
    );
    assert_eq!(screen, Screen::WaitingForSignIn(Account::Anthropic));
}

#[test]
fn going_back_from_the_account_sign_in_selects_it_again() {
    let mut screen = Screen::WaitingForSignIn(Account::Anthropic);

    assert_eq!(screen.input(Input::Back), Command::CancelSignIn);
    assert_eq!(screen, Screen::choosing(Method::AnthropicAccount));
}

#[test]
fn an_account_sign_in_connects_with_that_account_or_explains_why_not() {
    let mut connected = Screen::WaitingForSignIn(Account::Anthropic);
    let mut failed = Screen::WaitingForSignIn(Account::Anthropic);

    connected.report(Report::SignedIn(Ok(())));
    failed.report(Report::SignedIn(Err(ConnectError::new(
        ConnectFailure::AntMissing,
    ))));

    assert_eq!(connected, Screen::Connected(Connection::AnthropicAccount));
    assert_eq!(
        failed,
        Screen::Choosing {
            selected: Method::AnthropicAccount,
            problem: Some(Problem::plain(Text::SignInAntMissing)),
        }
    );
}

#[test]
fn pointing_at_a_way_selects_it() {
    let mut screen = Screen::choosing(Method::ClaudeCode);

    screen.input(Input::Point(Method::ApiKey));

    assert_eq!(screen, Screen::choosing(Method::ApiKey));
}

#[test]
fn confirming_the_claude_account_signs_in_through_claude_code() {
    let mut screen = Screen::choosing(Method::ClaudeCode);

    assert_eq!(
        screen.input(Input::Confirm),
        Command::SignIn(Account::ClaudeCode)
    );
    assert_eq!(screen, Screen::WaitingForSignIn(Account::ClaudeCode));
}

#[test]
fn confirming_the_api_key_asks_for_it() {
    let mut screen = Screen::choosing(Method::ApiKey);

    assert_eq!(screen.input(Input::Confirm), Command::Nothing);
    assert_eq!(screen, entering(""));
}

#[test]
fn pressing_a_way_chooses_it_at_once() {
    let mut screen = Screen::choosing(Method::ClaudeCode);

    screen.input(Input::Press(Method::ApiKey));

    assert_eq!(screen, entering(""));
}

#[test]
fn numbers_choose_a_way_like_in_claude_code() {
    let mut screen = Screen::choosing(Method::ApiKey);
    assert_eq!(
        screen.input(type_text("1")),
        Command::SignIn(Account::ClaudeCode)
    );

    let mut screen = Screen::choosing(Method::ClaudeCode);
    assert_eq!(
        screen.input(type_text("2")),
        Command::SignIn(Account::Anthropic)
    );

    let mut screen = Screen::choosing(Method::ClaudeCode);
    screen.input(type_text("3"));
    assert_eq!(screen, entering(""));

    let mut screen = Screen::choosing(Method::ClaudeCode);
    screen.input(type_text("4"));
    assert_eq!(screen, Screen::choosing(Method::ClaudeCode));
}

#[test]
fn typing_and_pasting_build_the_key_without_blanks() {
    let mut screen = entering("");

    screen.input(type_text("sk-"));
    screen.input(Input::Paste(" ant-api03\n".to_owned()));

    assert_eq!(screen, entering("sk-ant-api03"));
}

#[test]
fn erasing_removes_the_last_character_only() {
    let mut screen = entering("sk-ant");

    screen.input(Input::Erase);

    assert_eq!(screen, entering("sk-an"));
}

#[test]
fn caps_the_draft_length() {
    let mut screen = entering("");

    screen.input(Input::Paste("a".repeat(LONGEST_DRAFT + 10)));
    screen.input(type_text("b"));

    let Screen::EnteringKey { draft, .. } = screen else {
        panic!("still entering the key");
    };
    assert_eq!(draft, "a".repeat(LONGEST_DRAFT));
}

#[test]
fn confirming_a_well_shaped_key_checks_it() {
    let mut screen = entering(KEY);

    let command = screen.input(Input::Confirm);

    assert_eq!(command, Command::CheckKey(ApiKey::parse(KEY).unwrap()));
    assert_eq!(screen, checking_key());
}

#[test]
fn confirming_a_malformed_key_explains_why_and_keeps_it() {
    let mut screen = entering("sk-proj-123");

    assert_eq!(screen.input(Input::Confirm), Command::Nothing);
    assert_eq!(
        screen,
        Screen::EnteringKey {
            draft: "sk-proj-123".to_owned(),
            problem: Some(Problem::plain(Text::SignInKeyMalformed)),
        }
    );
}

#[test]
fn editing_the_key_clears_the_problem() {
    for edit in [Input::Erase, type_text("x")] {
        let mut screen = Screen::EnteringKey {
            draft: "sk".to_owned(),
            problem: Some(Problem::plain(Text::SignInKeyMalformed)),
        };

        screen.input(edit);

        assert!(matches!(screen, Screen::EnteringKey { problem: None, .. }));
    }
}

#[test]
fn going_back_from_the_key_returns_to_the_choice() {
    let mut screen = entering("sk-ant");

    screen.input(Input::Back);

    assert_eq!(screen, Screen::choosing(Method::ApiKey));
}

#[test]
fn going_back_while_claude_code_signs_in_cancels_it() {
    let mut screen = Screen::WaitingForSignIn(Account::ClaudeCode);

    assert_eq!(screen.input(Input::Back), Command::CancelSignIn);
    assert_eq!(screen, Screen::choosing(Method::ClaudeCode));
}

#[test]
fn ignores_keys_while_waiting() {
    for mut screen in [
        checking_key(),
        Screen::WaitingForSignIn(Account::ClaudeCode),
        Screen::Checking,
    ] {
        let before = format!("{screen:?}");

        for input in [Input::Confirm, Input::Next, type_text("1"), Input::Erase] {
            assert_eq!(screen.input(input), Command::Nothing);
        }

        assert_eq!(format!("{screen:?}"), before);
    }
}

#[test]
fn opens_dot_when_something_is_connected() {
    let mut screen = Screen::Checking;

    screen.report(Report::Checked(Ok(Some(Connection::ApiKey))));

    assert_eq!(screen, Screen::Connected(Connection::ApiKey));
}

#[test]
fn remembers_a_claude_account_found_at_start() {
    let mut screen = Screen::Checking;

    screen.report(Report::Checked(Ok(Some(Connection::ClaudeCode))));

    assert_eq!(screen, Screen::Connected(Connection::ClaudeCode));
}

#[test]
fn offers_both_ways_when_nothing_is_connected() {
    let mut screen = Screen::Checking;

    screen.report(Report::Checked(Ok(None)));

    assert_eq!(screen, Screen::choosing(Method::ClaudeCode));
}

#[test]
fn offers_both_ways_with_the_problem_when_the_check_fails() {
    let mut screen = Screen::Checking;
    let error = ConnectError::caused_by(
        ConnectFailure::StorageFailed,
        io::Error::other("keychain locked"),
    );

    screen.report(Report::Checked(Err(error)));

    assert_eq!(
        screen,
        Screen::Choosing {
            selected: Method::ClaudeCode,
            problem: Some(Problem {
                text: Text::SignInStorageFailed,
                cause: Some("keychain locked".to_owned()),
            }),
        }
    );
}

#[test]
fn opens_dot_once_the_key_is_accepted() {
    let mut screen = checking_key();

    screen.report(Report::KeyChecked(Ok(())));

    assert_eq!(screen, Screen::Connected(Connection::ApiKey));
}

#[test]
fn gives_the_key_back_when_anthropic_rejects_it() {
    let mut screen = checking_key();

    screen.report(Report::KeyChecked(Err(failure(
        ConnectFailure::KeyRejected,
    ))));

    assert_eq!(
        screen,
        Screen::EnteringKey {
            draft: KEY.to_owned(),
            problem: Some(Problem::plain(Text::SignInKeyRejected)),
        }
    );
}

#[test]
fn opens_dot_once_claude_code_is_signed_in() {
    let mut screen = Screen::WaitingForSignIn(Account::ClaudeCode);

    screen.report(Report::SignedIn(Ok(())));

    assert_eq!(screen, Screen::Connected(Connection::ClaudeCode));
}

#[test]
fn returns_to_the_choice_when_claude_code_fails() {
    let mut screen = Screen::WaitingForSignIn(Account::ClaudeCode);

    screen.report(Report::SignedIn(Err(failure(
        ConnectFailure::ClaudeCodeMissing,
    ))));

    assert_eq!(
        screen,
        Screen::Choosing {
            selected: Method::ClaudeCode,
            problem: Some(Problem::plain(Text::SignInClaudeCodeMissing)),
        }
    );
}

#[test]
fn ignores_a_report_that_arrives_too_late() {
    let mut screen = Screen::choosing(Method::ClaudeCode);

    screen.report(Report::SignedIn(Ok(())));
    screen.report(Report::KeyChecked(Ok(())));
    screen.report(Report::Checked(Ok(Some(Connection::ClaudeCode))));

    assert_eq!(screen, Screen::choosing(Method::ClaudeCode));
}

proptest! {
    #[test]
    fn the_draft_holds_only_visible_ascii(pasted in any::<String>()) {
        let mut screen = entering("");

        screen.input(Input::Paste(pasted));

        let Screen::EnteringKey { draft, .. } = screen else {
            panic!("still entering the key");
        };
        prop_assert!(draft.len() <= LONGEST_DRAFT);
        prop_assert!(draft.chars().all(|character| character.is_ascii_graphic()));
    }
}
