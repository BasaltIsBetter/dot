use super::*;

#[test]
fn keeps_only_the_end_of_a_long_last_line_of_what_claude_code_wrote() {
    let long = format!("{}é{}", "a".repeat(500), "z".repeat(300));
    let ClaudeCodeError::NoResultSaying(_, kept) = no_result(
        std::process::ExitStatus::default(),
        &format!("first\n{long}\n  \n"),
    ) else {
        panic!("nothing was kept");
    };

    assert!(kept.len() <= 400 + 3, "{}", kept.len());
    assert!(kept.ends_with(&"z".repeat(300)));
    assert!(matches!(
        no_result(std::process::ExitStatus::default(), " \n "),
        ClaudeCodeError::NoResult(_)
    ));
}
