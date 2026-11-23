use std::fmt::Write;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use dot_api::{Detail, FileChange, Hunk, PatchLine, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::dpi::PhysicalPosition;
use winit::keyboard::NamedKey;

use dot_transcript::Ink;

use super::*;
use crate::app::tests::{
    IdTranslator, TestApp, act, canvas_of, chatting, named, pane_texts, settled, shown_strip, typed,
};
use crate::canvas::{Rectangle, Rgba};
use crate::editor::tests::MemoryWorkspace;
use crate::event::Delivery;
use crate::reply::tests::ScriptedChat;

const WAIT: Duration = Duration::from_secs(5);
const FILE: &str = "a\nB\nc";

fn with_files(files: &[(&str, &str)]) -> (TestApp, Receiver<Delivery>, Arc<MemoryWorkspace>) {
    let (mut app, receiver) = chatting(ScriptedChat::default());
    let workspace = Arc::new(MemoryWorkspace::with(files));
    app.editing = Editing::new(Arc::clone(&workspace), app.mailbox.clone());
    (app, receiver, workspace)
}

fn claude_changed(app: &mut TestApp, path: &str) {
    act(app, typed("f"));
    act(app, named(NamedKey::Enter));
    let now = Instant::now();
    let conversation = &mut app.sessions.active_mut().conversation;
    let tool = ToolUse {
        kind: ToolKind::Edit,
        subject: path.to_owned(),
    };
    let change = FileChange {
        path: path.to_owned(),
        created: false,
        hunks: vec![Hunk {
            old_start: 1,
            new_start: 1,
            lines: vec![
                PatchLine::Same("a".to_owned()),
                PatchLine::Removed("b".to_owned()),
                PatchLine::Added("B".to_owned()),
                PatchLine::Same("c".to_owned()),
            ],
        }],
        siblings: None,
    };
    let end = ToolEnd::Done(Detail::Changes {
        added: 1,
        removed: 1,
    });
    for event in [
        ReplyEvent::ToolStarted {
            id: "t1".to_owned(),
            tool,
        },
        ReplyEvent::ToolChanged {
            id: "t1".to_owned(),
            change,
        },
        ReplyEvent::ToolEnded {
            id: "t1".to_owned(),
            end,
        },
    ] {
        conversation.hear(event, now, &IdTranslator);
    }
    act(app, WindowAction::Paint);
}

fn press_on(app: &mut TestApp, text: &str) {
    let layout = app.layout();
    let placed = canvas_of(app)
        .shown
        .panes
        .iter()
        .flat_map(|pane| pane.lines.iter().map(move |placed| (pane, placed)))
        .find(|(_, placed)| {
            placed
                .line
                .spans
                .iter()
                .any(|span| span.text.contains(text))
        })
        .map_or_else(
            || panic!("{text} is not shown"),
            |(pane, placed)| {
                let indent = f32::from(placed.line.indent) * layout.advance_of(pane.size);
                (pane.left + indent, placed.top)
            },
        );
    let at = PhysicalPosition::new(f64::from(placed.0) + 4.0, f64::from(placed.1) + 6.0);
    act(app, WindowAction::Point(at));
    act(app, WindowAction::Press);
}

fn next_disk_news(app: &mut TestApp, receiver: &Receiver<Delivery>) {
    loop {
        let delivery = receiver.recv_timeout(WAIT).unwrap();
        let disk = matches!(delivery, Delivery::Disk(_));
        app.deliver(delivery, Instant::now());
        if disk {
            act(app, WindowAction::Paint);
            return;
        }
    }
}

fn opened(files: &[(&str, &str)]) -> (TestApp, Receiver<Delivery>, Arc<MemoryWorkspace>) {
    let (mut app, receiver, workspace) = with_files(files);
    claude_changed(&mut app, "src/a.rs");
    press_on(&mut app, "↗");
    next_disk_news(&mut app, &receiver);
    (app, receiver, workspace)
}

#[test]
fn opens_a_file_claude_changed_from_its_name_in_the_conversation() {
    let (app, _receiver, _) = opened(&[("src/a.rs", FILE), ("src/b.rs", "")]);

    assert!(app.editing.editor.is_some());
    assert_eq!(pane_texts(&app, 0), ["   1", "── 2", "   3"]);
    assert_eq!(&pane_texts(&app, 1)[..3], ["a", "B", "c"]);
    assert!(shown_strip(&app).contains("a.rs · editor-saved"));
    assert!(canvas_of(&app).shown.animating);
}

#[test]
fn saves_each_keystroke_and_says_so() {
    let (mut app, receiver, workspace) = opened(&[("src/a.rs", FILE)]);

    act(&mut app, typed("x"));
    act(&mut app, WindowAction::Commit("é".to_owned()));
    next_disk_news(&mut app, &receiver);
    while workspace.text("src/a.rs").as_deref() != Some("xéa\nB\nc") {
        next_disk_news(&mut app, &receiver);
    }

    assert_eq!(&pane_texts(&app, 1)[..1], ["xéa"]);
    assert_eq!(pane_texts(&app, 0)[0], "── 1");
}

#[test]
fn says_when_a_keystroke_could_not_be_saved_and_ignores_news_of_another_file() {
    let (mut app, receiver, _) = opened(&[("src/a.rs", FILE)]);
    let now = Instant::now();

    app.disk_news(
        DiskNews::Saved {
            path: "src/b.rs".to_owned(),
            result: Err(dot_api::WorkspaceError::TooLarge),
        },
        now,
    );
    act(&mut app, WindowAction::Paint);
    assert!(shown_strip(&app).contains("editor-saved"));
    app.disk_news(
        DiskNews::Saved {
            path: "src/a.rs".to_owned(),
            result: Err(dot_api::WorkspaceError::TooLarge),
        },
        now,
    );
    act(&mut app, WindowAction::Paint);
    assert!(shown_strip(&app).contains("a.rs · editor-not-saved"));
    drop(receiver);
}

#[test]
fn escape_brings_the_conversation_back_in_a_fade() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);

    act(&mut app, named(NamedKey::Escape));

    assert!(app.editing.editor.is_none());
    assert!(canvas_of(&app).shown.animating);
    assert!(
        canvas_of(&app).shown.panes[0]
            .lines
            .iter()
            .any(|placed| placed.line.spans.iter().any(|span| span.text.contains("↗")))
    );
    act(&mut app, typed("y"));
    assert!(app.editing.editor.is_none());
}

#[test]
fn the_conversation_comes_back_from_a_little_larger() {
    let (mut app, _receiver, _) = with_files(&[]);
    let start = Instant::now();
    app.editing.left = Some(start);
    let mark = Rectangle {
        left: 1_100.0,
        top: 100.0,
        width: 1.0,
        height: 1.0,
        color: Rgba::ink(Ink::Full),
    };
    let mut scene = Scene {
        marks: vec![mark],
        ..Scene::default()
    };

    app.fade_back(
        &mut scene,
        [100.0, 100.0],
        start + Duration::from_millis(100),
    );

    let left = scene.marks[0].left;
    assert!(left > 1_100.0 && left < 100.0 + 1_000.0 * 1.03, "{left}");
    assert_ne!(scene.marks[0].color, mark.color);
    assert!(scene.animating);
}

#[test]
fn leaves_a_notice_when_a_file_will_not_open() {
    let (mut app, receiver, _) = with_files(&[]);
    claude_changed(&mut app, "src/a.rs");
    press_on(&mut app, "↗");
    next_disk_news(&mut app, &receiver);

    assert!(app.editing.editor.is_none());
    let notices = canvas_of(&app).shown.panes[0]
        .lines
        .iter()
        .filter(|placed| {
            placed
                .line
                .spans
                .iter()
                .any(|span| span.text.contains("editor-not-a-file"))
        })
        .count();
    assert_eq!(notices, 1);
}

#[test]
fn clicks_close_the_editor_and_open_another_file_claude_touched() {
    let (mut app, receiver, _) = opened(&[("src/a.rs", FILE), ("src/b.rs", "b")]);
    let now = Instant::now();
    let mut touched = crate::editor::Touched::default();
    touched.record(&FileChange::between("src/b.rs", "", "b", true));
    let editor = Editor::new(
        "src/a.rs".to_owned(),
        FILE.to_owned(),
        &touched,
        vec!["a.rs".to_owned(), "b.rs".to_owned()],
        now,
    );
    app.editing.editor = Some(editor);
    act(&mut app, WindowAction::Paint);

    press_on(&mut app, "b.rs");
    next_disk_news(&mut app, &receiver);
    assert_eq!(
        app.editing.editor.as_ref().map(Editor::path),
        Some("src/b.rs")
    );

    press_on(&mut app, "editor-back");
    assert!(app.editing.editor.is_none());
}

#[test]
fn the_wheel_scrolls_the_code_and_a_click_places_the_caret() {
    let long = (1..=200).fold(String::new(), |mut long, line| {
        writeln!(long, "line {line}").unwrap();
        long
    });
    let (mut app, _receiver, _) = opened(&[("src/a.rs", &long)]);

    act(&mut app, WindowAction::Scroll(Wheel::Lines(-2.0)));
    assert_eq!(pane_texts(&app, 0)[0], "     7");
    act(&mut app, WindowAction::Scroll(Wheel::Pixels(35.2 * 3.0)));
    assert_eq!(pane_texts(&app, 0)[0], "     4");

    settled(&mut app);
    press_on(&mut app, "line 5");
    act(&mut app, typed("!"));
    assert_eq!(pane_texts(&app, 1)[1], "!line 5");
}

#[test]
fn takes_nothing_while_closed_and_nothing_but_chat_input() {
    let (mut app, _receiver, _) = with_files(&[]);
    let now = Instant::now();

    assert!(!app.edit_input(&WindowAction::Press, now));
    let (mut opened_app, _other, _) = opened(&[("src/a.rs", FILE)]);
    assert!(!opened_app.edit_input(&WindowAction::Focus, now));
    assert!(opened_app.edit_input(&WindowAction::Press, now));
    opened_app.screen = Screen::SignIn;
    assert!(!opened_app.edit_input(&WindowAction::Press, now));
    assert!(app.editor_scene(&app.layout(), now).is_none());
}
