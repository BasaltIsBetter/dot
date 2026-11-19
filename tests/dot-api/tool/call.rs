use super::*;

fn path(text: &str) -> String {
    text.to_owned()
}

#[test]
fn shows_what_each_call_does_and_on_what() {
    let calls = [
        (
            ToolCall::View {
                path: path("src/main.rs"),
                lines: Some(LineRange {
                    first: 1,
                    last: None,
                }),
            },
            ToolKind::Read,
            "src/main.rs",
        ),
        (
            ToolCall::Create {
                path: path("new.rs"),
                text: path("fn main() {}"),
            },
            ToolKind::Write,
            "new.rs",
        ),
        (
            ToolCall::Replace {
                path: path("a.rs"),
                old: path("x"),
                new: path("y"),
            },
            ToolKind::Edit,
            "a.rs",
        ),
        (
            ToolCall::Insert {
                path: path("b.rs"),
                after_line: 3,
                text: path("z"),
            },
            ToolKind::Edit,
            "b.rs",
        ),
        (
            ToolCall::Run {
                command: path("cargo test"),
            },
            ToolKind::Run,
            "cargo test",
        ),
        (ToolCall::RestartShell, ToolKind::Run, "restart"),
        (
            ToolCall::FindFiles {
                pattern: path("**/*.rs"),
                folder: None,
            },
            ToolKind::FindFiles,
            "**/*.rs",
        ),
        (
            ToolCall::SearchText(TextSearch {
                pattern: path("fn main"),
                folder: Some(path("src")),
                files: None,
                ignore_case: false,
                show: SearchShow::default(),
            }),
            ToolKind::SearchText,
            "fn main",
        ),
    ];

    for (call, kind, subject) in calls {
        assert_eq!(
            call.shown(),
            ToolUse {
                kind,
                subject: subject.to_owned()
            }
        );
    }
}

#[test]
fn a_search_gives_back_the_matching_files_unless_told_otherwise() {
    assert_eq!(SearchShow::default(), SearchShow::Files);
}
