use std::borrow::Cow;

use dot_api::{Detail, ToolKind};
use proptest::prelude::*;

use super::*;
use crate::conversation::flow::tests::IdTranslator;

struct CountingTranslator;

impl Translator for CountingTranslator {
    fn use_language(&self, _language: dot_api::Language) {}

    fn translate(&self, text: Text) -> Cow<'_, str> {
        Cow::Borrowed(text.id())
    }

    fn count(&self, text: Text, count: u64) -> Cow<'_, str> {
        let form = if count == 1 { "one" } else { "other" };
        Cow::Owned(format!("{}:{form}", text.id()))
    }
}

fn tool(kind: ToolKind, subject: &str) -> ToolUse {
    ToolUse {
        kind,
        subject: subject.to_owned(),
    }
}

fn label(kind: ToolKind, subject: &str) -> (Glyph, String) {
    let started = started(&tool(kind, subject), &IdTranslator);
    (started.glyph, started.label)
}

#[test]
fn names_a_file_by_its_name_only() {
    assert_eq!(
        label(ToolKind::Read, "crates/a/src/main.rs"),
        (Glyph::Read, "main.rs".to_owned())
    );
    assert_eq!(
        label(ToolKind::Edit, "src/lib.rs"),
        (Glyph::Edit, "lib.rs".to_owned())
    );
    assert_eq!(
        label(ToolKind::Write, "notes/"),
        (Glyph::Create, "notes".to_owned())
    );
    assert_eq!(
        label(ToolKind::Read, "README.md"),
        (Glyph::Read, "README.md".to_owned())
    );
}

#[test]
fn quotes_what_a_search_looks_for() {
    assert_eq!(
        label(ToolKind::SearchText, "on_resize"),
        (Glyph::Search, "quote-openon_resizequote-close".to_owned())
    );
    assert_eq!(label(ToolKind::FindFiles, "*.rs").0, Glyph::Search);
}

#[test]
fn shows_a_command_a_page_and_an_agent_by_what_they_work_on() {
    assert_eq!(
        label(ToolKind::Run, "cargo test\nls"),
        (Glyph::Command, "cargo test".to_owned())
    );
    assert_eq!(
        label(ToolKind::Fetch, "https://docs.rs/winit"),
        (Glyph::Web, "docs.rs/winit".to_owned())
    );
    assert_eq!(label(ToolKind::Fetch, "http://a.b").1, "a.b");
    assert_eq!(
        label(ToolKind::SearchWeb, "winit resize"),
        (Glyph::Web, "winit resize".to_owned())
    );
    assert_eq!(label(ToolKind::Delegate, "find the grid").0, Glyph::Agents);
    assert_eq!(label(ToolKind::Other, "Skill review").0, Glyph::Other);
}

#[test]
fn cuts_a_very_long_subject() {
    let (_, shown) = label(ToolKind::Run, &"x".repeat(1_000));

    assert_eq!(shown.chars().count(), 240);
    assert!(shown.ends_with('…'));
    assert!(label(ToolKind::Run, &"x".repeat(240)).1.ends_with('x'));
}

#[test]
fn measures_how_each_tool_ended_in_a_few_columns() {
    let measured = |end| result(end, &CountingTranslator);

    assert_eq!(measured(ToolEnd::Done(Detail::Nothing)), "✓");
    assert_eq!(measured(ToolEnd::Done(Detail::Exit(0))), "✓");
    assert_eq!(measured(ToolEnd::Done(Detail::Exit(2))), "✗ 2");
    assert_eq!(
        measured(ToolEnd::Done(Detail::Lines(212))),
        "212 gutter-lines"
    );
    assert_eq!(measured(ToolEnd::Done(Detail::Files(4))), "×4");
    assert_eq!(measured(ToolEnd::Done(Detail::Matches(3))), "×3");
    assert_eq!(
        measured(ToolEnd::Done(Detail::Changes {
            added: 2,
            removed: 1
        })),
        "+2 −1"
    );
    assert_eq!(measured(ToolEnd::Failed), "tool-failed");
    assert_eq!(measured(ToolEnd::Refused), "tool-refused");
    assert_eq!(measured(ToolEnd::Stopped), "tool-stopped");
}

#[test]
fn sums_up_a_command_with_its_result_and_how_long_it_took() {
    let command = started(&tool(ToolKind::Run, "cargo test"), &IdTranslator);
    let read = started(&tool(ToolKind::Read, "a.rs"), &IdTranslator);
    let took = std::time::Duration::from_millis(1_260);

    let summed = aside(&command, ("✓", took), &IdTranslator);

    assert_eq!(
        summed,
        Some(Summed {
            cut: "⌘ cargo test".to_owned(),
            kept: "  ✓ · 1decimal-separator3 unit-seconds".to_owned(),
        })
    );
    assert_eq!(aside(&read, ("1 l.", took), &IdTranslator), None);
}

#[test]
fn hands_the_command_over_to_be_cut_and_the_rest_to_be_kept_whole() {
    let summed = Summed {
        cut: "⌘ ls".to_owned(),
        kept: "  ✓".to_owned(),
    };
    let agent = Summed::whole("⧉ explore · 3 s".to_owned());

    assert_eq!(
        summed.as_aside(),
        Aside {
            cut: "⌘ ls",
            kept: "  ✓"
        }
    );
    assert_eq!(
        agent.as_aside(),
        Aside {
            cut: "",
            kept: "⧉ explore · 3 s"
        }
    );
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

proptest! {
    #[test]
    fn labels_a_subject_as_its_whole_first_line_would(
        first in "[a-z \\t\\r\u{200b}é]{0,300}",
        rest in "[a-z\\n]{0,40}",
    ) {
        let subject = format!("{first}\n{rest}");
        let whole = revealed(&subject);
        let expected = truncated(whole.split('\n').next().unwrap_or_default(), SUBJECT_MOST_CHARACTERS);

        prop_assert_eq!(first_line(&subject), expected);
    }
}
