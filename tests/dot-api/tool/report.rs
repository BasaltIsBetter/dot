use super::*;

#[test]
fn names_each_kind_of_tool_with_its_own_text() {
    let kinds = [
        (ToolKind::Read, Text::ToolRead),
        (ToolKind::Write, Text::ToolWrite),
        (ToolKind::Edit, Text::ToolEdit),
        (ToolKind::Run, Text::ToolRun),
        (ToolKind::FindFiles, Text::ToolFindFiles),
        (ToolKind::SearchText, Text::ToolSearchText),
        (ToolKind::Fetch, Text::ToolFetch),
        (ToolKind::SearchWeb, Text::ToolSearchWeb),
        (ToolKind::Delegate, Text::ToolDelegate),
        (ToolKind::Other, Text::ToolOther),
    ];

    for (kind, text) in kinds {
        assert_eq!(kind.text(), text);
    }
}

#[test]
fn a_tool_that_ran_is_no_error() {
    let output = ToolOutput::done("3 lines", Detail::Lines(3));

    assert_eq!(output.content, ToolContent::Text("3 lines".to_owned()));
    assert!(!output.is_error);
    assert_eq!(output.end, ToolEnd::Done(Detail::Lines(3)));
}

#[test]
fn a_tool_that_failed_tells_claude_why_as_an_error() {
    let output = ToolOutput::failed("no such file");

    assert_eq!(output.content, ToolContent::Text("no such file".to_owned()));
    assert!(output.is_error);
    assert_eq!(output.end, ToolEnd::Failed);
}

#[test]
fn a_refused_or_stopped_tool_tells_claude_what_the_user_did() {
    let refused = ToolOutput::refused();
    let stopped = ToolOutput::stopped();

    assert_eq!(
        refused.content,
        ToolContent::Text(USER_REFUSED_THE_TOOL.to_owned())
    );
    assert!(refused.is_error);
    assert_eq!(refused.end, ToolEnd::Refused);
    assert_eq!(
        stopped.content,
        ToolContent::Text(USER_STOPPED_THE_REPLY.to_owned())
    );
    assert!(stopped.is_error);
    assert_eq!(stopped.end, ToolEnd::Stopped);
}

#[test]
fn measures_nothing_unless_told() {
    assert_eq!(Detail::default(), Detail::Nothing);
}
