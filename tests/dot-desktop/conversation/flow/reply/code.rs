use std::time::Instant;

use dot_api::{Detail, Hunk, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::dpi::PhysicalPosition;

use super::*;
use crate::conversation::flow::Conversation;
use crate::conversation::flow::tests::{
    IdTranslator, after, flow_texts, layout, scene_at, sent, stage, text_of,
};

fn change(lines: Vec<PatchLine>) -> FileChange {
    FileChange {
        path: "src/a.rs".to_owned(),
        created: false,
        hunks: vec![Hunk {
            old_start: 1,
            new_start: 1,
            lines,
        }],
        siblings: None,
    }
}

fn folding() -> FileChange {
    let mut lines: Vec<PatchLine> = (1..=8)
        .map(|line| PatchLine::Same(format!("line {line}")))
        .collect();
    lines.push(PatchLine::Removed("old".to_owned()));
    lines.push(PatchLine::Added("new".to_owned()));
    change(lines)
}

fn edited(start: Instant) -> Conversation {
    let mut conversation = sent("fix", start);
    let hear =
        |conversation: &mut Conversation, event| conversation.hear(event, start, &IdTranslator);
    hear(&mut conversation, ReplyEvent::Text("Fixing.".to_owned()));
    let tool = ToolUse {
        kind: ToolKind::Edit,
        subject: "src/a.rs".to_owned(),
    };
    hear(
        &mut conversation,
        ReplyEvent::ToolStarted {
            id: "t1".to_owned(),
            tool,
        },
    );
    hear(
        &mut conversation,
        ReplyEvent::ToolChanged {
            id: "t1".to_owned(),
            change: Box::new(folding()),
        },
    );
    let end = ToolEnd::Done(Detail::Changes {
        added: 1,
        removed: 1,
    });
    hear(
        &mut conversation,
        ReplyEvent::ToolEnded {
            id: "t1".to_owned(),
            end,
        },
    );
    conversation
}

fn top_of(conversation: &mut Conversation, text: &str, now: Instant) -> f32 {
    let scene = scene_at(conversation, now);
    scene.panes[0]
        .lines
        .iter()
        .find(|placed| text_of(placed).contains(text))
        .map_or_else(
            || panic!("{text} not in {:?}", flow_texts(&scene)),
            |placed| placed.top,
        )
}

fn on_the_column(y: f32) -> [f32; 2] {
    [stage().column[0] + 40.0, y + stage().row / 2.0]
}

#[test]
fn says_its_words_in_the_language_of_the_interface() {
    let words = Words(&IdTranslator);

    assert_eq!(words.lines(3), "tool-lines");
    assert_eq!(words.new_file(), "code-new");
    assert_eq!(words.fold_back(), "code-fold");
}

#[test]
fn shows_what_is_hidden_or_reversed_and_turns_tabs_into_spaces() {
    let shown = change_of(&FileChange {
        path: "a\u{202e}.rs".to_owned(),
        siblings: Some(Siblings {
            folder: "src\u{200b}/".to_owned(),
            before: vec!["b\u{2066}.rs".to_owned()],
            after: vec!["c.rs".to_owned()],
        }),
        ..change(vec![
            PatchLine::Same("\tx".to_owned()),
            PatchLine::Removed("a\u{200b}".to_owned()),
            PatchLine::Added("b".to_owned()),
        ])
    });

    assert_eq!(shown.path, "a\u{fffd}.rs");
    assert_eq!(
        shown.hunks[0].lines,
        [
            ChangeLine::Same("    x".to_owned()),
            ChangeLine::Removed("a\u{fffd}".to_owned()),
            ChangeLine::Added("b".to_owned()),
        ]
    );
    let neighbours = shown.neighbours.unwrap();
    assert_eq!(neighbours.folder, "src\u{fffd}/");
    assert_eq!(neighbours.before, ["b\u{fffd}.rs"]);
    assert_eq!(neighbours.after, ["c.rs"]);
}

#[test]
fn keeps_the_count_of_lines_past_those_the_conversation_shows() {
    let first = (0..CODE_LINES_KEPT)
        .map(|line| PatchLine::Added(line.to_string()))
        .collect();
    let second = vec![
        PatchLine::Added("past".to_owned()),
        PatchLine::Same("too".to_owned()),
    ];
    let shown = change_of(&FileChange {
        hunks: vec![
            Hunk {
                old_start: 1,
                new_start: 1,
                lines: first,
            },
            Hunk {
                old_start: 900,
                new_start: 900,
                lines: second,
            },
        ],
        ..change(Vec::new())
    });

    assert_eq!(
        shown.hunks[0].lines[CODE_LINES_KEPT - 1],
        ChangeLine::Added("399".to_owned())
    );
    assert_eq!(
        shown.hunks[1].lines,
        [
            ChangeLine::Added(String::new()),
            ChangeLine::Same(String::new())
        ]
    );
    assert_eq!(shown.hunks[1].new_start, 900);
    assert!(!shown.created);
}

#[test]
fn shows_the_lines_a_tool_changed_under_it() {
    let start = Instant::now();
    let mut conversation = edited(start);

    let texts = flow_texts(&scene_at(&mut conversation, after(start, 5_000)));

    assert!(texts.contains(&"a.rs ↗".to_owned()), "{texts:?}");
    assert!(
        texts.contains(&"   · │ ··· 6 tool-lines".to_owned()),
        "{texts:?}"
    );
    assert!(texts.contains(&"── 9 │ new".to_owned()), "{texts:?}");
}

#[test]
fn opens_and_closes_folded_lines_on_a_click() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let mut conversation = edited(start);
    let fold = top_of(&mut conversation, "··· 6", later);
    let [x, y] = on_the_column(fold);

    conversation.press(
        PhysicalPosition::new(f64::from(x), f64::from(y)),
        &layout(),
        later,
    );
    let opened = flow_texts(&scene_at(&mut conversation, later));
    conversation.press(
        PhysicalPosition::new(f64::from(x), f64::from(y)),
        &layout(),
        later,
    );
    let closed = flow_texts(&scene_at(&mut conversation, later));

    assert!(opened.contains(&"   1 │ line 1".to_owned()), "{opened:?}");
    assert!(!closed.contains(&"   1 │ line 1".to_owned()), "{closed:?}");
}

#[test]
fn shows_the_old_line_wherever_the_pointer_is_on_its_row() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let mut conversation = edited(start);
    let changed = top_of(&mut conversation, "── 9", later);
    let [x, y] = on_the_column(changed);

    conversation.point(Some([x, y]));
    let hovered = flow_texts(&scene_at(&mut conversation, later));
    conversation.point(Some([1.0, y]));
    let beside = flow_texts(&scene_at(&mut conversation, later));
    conversation.point(None);
    let away = flow_texts(&scene_at(&mut conversation, later));

    assert!(hovered.contains(&"── 9 │ old".to_owned()), "{hovered:?}");
    assert!(beside.contains(&"── 9 │ old".to_owned()), "{beside:?}");
    assert!(away.contains(&"── 9 │ new".to_owned()), "{away:?}");
}

#[test]
fn keeps_as_much_of_a_code_line_as_a_window_can_show() {
    let mut room = 10;

    let line = line_of(&PatchLine::Added("x".repeat(2_000_000)), &mut room);

    let ChangeLine::Added(text) = line else {
        panic!("not an added line");
    };
    assert_eq!(text.chars().count(), CODE_COLUMNS);
    assert_eq!(CODE_COLUMNS, usize::from(dot_transcript::WIDEST_COLUMN) + 1);
}

fn edited_twice(start: Instant) -> Conversation {
    let mut conversation = edited(start);
    let hear =
        |conversation: &mut Conversation, event| conversation.hear(event, start, &IdTranslator);
    let tool = ToolUse {
        kind: ToolKind::Edit,
        subject: "src/b.rs".to_owned(),
    };
    hear(
        &mut conversation,
        ReplyEvent::ToolStarted {
            id: "t2".to_owned(),
            tool,
        },
    );
    hear(
        &mut conversation,
        ReplyEvent::ToolChanged {
            id: "t2".to_owned(),
            change: Box::new(FileChange {
                path: "src/b.rs".to_owned(),
                ..folding()
            }),
        },
    );
    conversation
}

fn at_column(y: f32, column: f32) -> PhysicalPosition<f64> {
    let x = stage().column[0] + layout().advance() * (column + 0.5);
    PhysicalPosition::new(f64::from(x), f64::from(y + stage().row / 2.0))
}

#[test]
fn folds_a_change_to_its_name_when_the_next_change_comes_right_after_it() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let mut conversation = edited_twice(start);

    let texts = flow_texts(&scene_at(&mut conversation, later));

    let header = texts.iter().position(|text| text.starts_with("a.rs ↗"));
    let header = header.unwrap_or_else(|| panic!("{texts:?}"));
    assert_eq!(texts[header], "a.rs ↗   ··· +1 −1");
    assert_eq!(texts[header + 1], "b.rs ↗");
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.as_str() == "── 9 │ new")
            .count(),
        1,
        "{texts:?}"
    );
}

#[test]
fn opens_a_folded_change_past_its_name_and_its_file_from_its_name() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let mut conversation = edited_twice(start);
    let header = top_of(&mut conversation, "a.rs ↗   ···", later);

    let file = conversation.file_under(at_column(header, 3.0), &layout(), later);
    let beside = conversation.file_under(at_column(header, 12.0), &layout(), later);
    conversation.press(at_column(header, 12.0), &layout(), later);
    let opened = flow_texts(&scene_at(&mut conversation, later));

    assert_eq!(file.as_deref(), Some("src/a.rs"));
    assert_eq!(beside, None);
    assert_eq!(
        opened
            .iter()
            .filter(|text| text.as_str() == "── 9 │ new")
            .count(),
        2,
        "{opened:?}"
    );
}

#[test]
fn opens_no_file_while_a_question_waits_for_an_answer() {
    let start = Instant::now();
    let later = after(start, 5_000);
    let mut conversation = edited(start);
    let name = top_of(&mut conversation, "a.rs ↗", later);
    let on_the_name = at_column(name, 3.0);
    let before = conversation.file_under(on_the_name, &layout(), later);
    let permission = dot_api::Permission {
        tool_id: "t9".to_owned(),
        action: dot_api::Action::Run {
            command: "ls".to_owned(),
        },
    };

    conversation.ask(&permission, later, &IdTranslator);

    assert_eq!(before.as_deref(), Some("src/a.rs"));
    assert_eq!(conversation.file_under(on_the_name, &layout(), later), None);
}
