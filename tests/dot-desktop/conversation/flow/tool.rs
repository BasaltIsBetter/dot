use std::borrow::Cow;

use dot_api::ToolKind;
use proptest::prelude::*;

use super::*;
use crate::conversation::flow::tests::IdTranslator;

struct CountingTranslator;

impl Translator for CountingTranslator {
    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }

    fn count(&self, text: Text, count: u64) -> Cow<'_, str> {
        let form = if count == 1 { "one" } else { "other" };
        Cow::Owned(format!("{}:{form}", text.id()))
    }
}

fn run(subject: &str) -> ToolUse {
    ToolUse {
        kind: ToolKind::Run,
        subject: subject.to_owned(),
    }
}

#[test]
fn shows_what_the_tool_does_in_front_of_its_subject() {
    let read = ToolUse {
        kind: ToolKind::Read,
        subject: "src/main.rs".to_owned(),
    };

    assert_eq!(started(&read, &IdTranslator), "tool-read src/main.rs");
    assert_eq!(
        started(&run("cargo test"), &IdTranslator),
        "tool-run cargo test"
    );
}

#[test]
fn shows_the_first_line_of_a_long_command_and_says_there_is_more() {
    assert_eq!(
        started(&run("cd src\ncargo test"), &IdTranslator),
        "tool-run cd src …"
    );
    assert_eq!(started(&run("ls\n"), &IdTranslator), "tool-run ls …");
}

#[test]
fn cuts_a_very_long_subject() {
    let shown = started(&run(&"x".repeat(1_000)), &IdTranslator);

    assert_eq!(shown.chars().count(), "tool-run ".len() + 240);
    assert!(shown.ends_with('…'));
    assert!(started(&run(&"x".repeat(240)), &IdTranslator).ends_with('x'));
    let cut_with_more = started(&run(&format!("{}\nls", "x".repeat(300))), &IdTranslator);
    assert!(cut_with_more.ends_with("x…"), "{cut_with_more}");
}

#[test]
fn measures_how_each_tool_ended() {
    let cases = [
        (ToolEnd::Done(Detail::Nothing), String::new()),
        (
            ToolEnd::Done(Detail::Lines(1)),
            "  1 tool-lines:one".to_owned(),
        ),
        (
            ToolEnd::Done(Detail::Files(12)),
            "  12 tool-files:other".to_owned(),
        ),
        (
            ToolEnd::Done(Detail::Matches(0)),
            "  0 tool-matches:other".to_owned(),
        ),
        (
            ToolEnd::Done(Detail::Changes {
                added: 3,
                removed: 1,
            }),
            "  +3 −1".to_owned(),
        ),
        (ToolEnd::Done(Detail::Exit(-2)), "  tool-exit -2".to_owned()),
        (ToolEnd::Failed, "  tool-failed".to_owned()),
        (ToolEnd::Refused, "  tool-refused".to_owned()),
        (ToolEnd::Stopped, "  tool-stopped".to_owned()),
    ];

    for (end, expected) in cases {
        assert_eq!(ended(end, &CountingTranslator), expected, "{end:?}");
    }
}

#[test]
fn reveals_the_characters_that_would_hide_or_reorder_text() {
    let hidden = [
        '\u{00ad}',
        '\u{061c}',
        '\u{180e}',
        '\u{200b}',
        '\u{200d}',
        '\u{200f}',
        '\u{2028}',
        '\u{202a}',
        '\u{202e}',
        '\u{2060}',
        '\u{2064}',
        '\u{2066}',
        '\u{2069}',
        '\u{206f}',
        '\u{feff}',
        '\u{fff9}',
        '\u{fffb}',
        '\u{e0000}',
        '\u{e0041}',
        '\u{e007f}',
    ];

    for character in hidden {
        assert_eq!(
            revealed(&format!("rm{character}x")),
            "rm\u{fffd}x",
            "{:x}",
            u32::from(character)
        );
    }
}

#[test]
fn keeps_what_shows_as_it_is() {
    let kept = [
        '\u{00ac}',
        '\u{00ae}',
        '\u{200a}',
        '\u{2027}',
        '\u{202f}',
        '\u{2065}',
        '\u{fefe}',
        '\u{fff8}',
        '\u{fffc}',
        '\u{dffff}',
        '\u{e0080}',
        'é',
        '→',
    ];

    for character in kept {
        let text = format!("a{character}b");
        assert_eq!(revealed(&text), text, "{:x}", u32::from(character));
    }
    assert_eq!(revealed("a\tb\r\nc\u{7}"), "a    b\nc");
}

proptest! {
    #[test]
    fn never_leaves_a_hidden_or_control_character(text in any::<String>()) {
        let shown = revealed(&text);

        for character in shown.chars() {
            prop_assert!(character == '\n' || !character.is_control());
            let hidden = INVISIBLE
                .iter()
                .any(|&(first, last)| (first..=last).contains(&character));
            prop_assert!(!hidden, "{:x}", u32::from(character));
        }
    }
}
