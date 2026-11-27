use std::fmt::Write;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use dot_api::{Detail, FileChange, Hunk, PatchLine, ReplyEvent, ToolEnd, ToolKind, ToolUse};
use winit::dpi::PhysicalPosition;
use winit::keyboard::NamedKey;

use dot_transcript::{Ink, opacity};

use super::*;
use crate::app::tests::{
    IdTranslator, TestApp, act, canvas_of, chatting, flow_texts, named, pane_texts, settled,
    shown_strip, typed,
};
use crate::canvas::{Paint, Rectangle};
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
            change: Box::new(change),
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
    let layout = app.layout(Instant::now());
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
    press_on(&mut app, "a.rs");
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
    assert!(canvas_of(&app).shown.panes[0].lines.iter().any(|placed| {
        placed
            .line
            .spans
            .iter()
            .any(|span| span.text.contains("a.rs"))
    }));
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
        color: Paint::ink(Ink::Full),
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
    press_on(&mut app, "a.rs");
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
    touched.record(FileChange::between("src/b.rs", "", "b", true));
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
    assert!(app.editor_scene(&app.layout(Instant::now()), now).is_none());
}

fn notices_saying(app: &TestApp, text: &str) -> usize {
    canvas_of(app).shown.panes[0]
        .lines
        .iter()
        .filter(|placed| {
            placed
                .line
                .spans
                .iter()
                .any(|span| span.text.contains(text))
        })
        .count()
}

#[test]
fn leaves_a_notice_when_a_file_not_in_the_editor_could_not_be_saved() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);
    act(&mut app, named(NamedKey::Escape));

    app.disk_news(
        DiskNews::Saved {
            path: "src/a.rs".to_owned(),
            result: Err(dot_api::WorkspaceError::Failed("disk full".to_owned())),
        },
        Instant::now(),
    );
    act(&mut app, WindowAction::Paint);

    assert_eq!(notices_saying(&app, "editor-lost src/a.rs"), 1);
}

#[test]
fn leaves_no_notice_when_a_file_was_saved() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);
    act(&mut app, named(NamedKey::Escape));

    app.disk_news(
        DiskNews::Saved {
            path: "src/a.rs".to_owned(),
            result: Ok(()),
        },
        Instant::now(),
    );
    act(&mut app, WindowAction::Paint);

    assert_eq!(notices_saying(&app, "editor-lost"), 0);
}

#[test]
fn leaves_a_notice_when_the_editor_closes_on_a_touch_up_not_saved() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);
    app.disk_news(
        DiskNews::Saved {
            path: "src/a.rs".to_owned(),
            result: Err(dot_api::WorkspaceError::Failed("disk full".to_owned())),
        },
        Instant::now(),
    );

    act(&mut app, named(NamedKey::Escape));
    act(&mut app, WindowAction::Paint);

    assert_eq!(notices_saying(&app, "editor-lost src/a.rs"), 1);
}

#[test]
fn finishing_writes_the_last_touch_up() {
    let (mut app, _receiver, workspace) = opened(&[("src/a.rs", FILE)]);
    act(&mut app, typed("z"));

    app.editing.finish();

    assert!(workspace.text("src/a.rs").unwrap().contains('z'));
}

#[test]
fn opens_the_file_asked_last_whichever_opening_ends_first() {
    let (mut app, _receiver, _) = with_files(&[("src/a.rs", FILE), ("src/b.rs", "b")]);
    let now = Instant::now();
    app.open_file("src/a.rs".to_owned());
    app.open_file("src/b.rs".to_owned());
    let opened = |path: &str, text: &str| DiskNews::Opened {
        path: path.to_owned(),
        text: Ok(text.to_owned()),
        entries: Vec::new(),
    };

    app.disk_news(opened("src/b.rs", "b"), now);
    app.disk_news(opened("src/a.rs", FILE), now);

    assert_eq!(
        app.editing.editor.as_ref().map(Editor::path),
        Some("src/b.rs")
    );
}

#[test]
fn ignores_a_file_opened_that_nobody_asked_for() {
    let (mut app, _receiver, _) = with_files(&[("src/a.rs", FILE)]);

    app.disk_news(
        DiskNews::Opened {
            path: "src/a.rs".to_owned(),
            text: Err(dot_api::WorkspaceError::NotAFile),
            entries: Vec::new(),
        },
        Instant::now(),
    );

    act(&mut app, WindowAction::Paint);

    assert!(app.editing.editor.is_none());
    assert!(
        !flow_texts(&app)
            .iter()
            .any(|text| text.contains("not-a-file"))
    );
}

#[test]
fn keeps_the_settings_closed_over_the_editor() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);

    app.open_settings(None, false, Instant::now());

    assert!(app.editing.is_open());
    assert!(!app.settings_screen.is_open());
}

#[test]
fn counts_the_sessions_in_the_bar_of_the_editor() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);

    app.open_session();
    app.sessions.switch_to(0);
    crate::app::tests::settled(&mut app);

    assert!(shown_strip(&app).contains("1/2"), "{}", shown_strip(&app));
}

fn on_code(app: &mut TestApp, row: usize, column: usize) -> WindowAction {
    crate::app::tests::settled(app);
    let layout = app.layout(Instant::now());
    let code = &canvas_of(app).shown.panes[1];
    let x = code.left + layout.advance() * (f32::from(u8::try_from(column).unwrap()) + 0.1);
    let y = code.lines[row].top + layout.line_height() / 2.0;
    WindowAction::Point(PhysicalPosition::new(f64::from(x), f64::from(y)))
}

fn selected_rows(app: &mut TestApp) -> Vec<Rectangle> {
    crate::app::tests::settled(app);
    let wash = Paint::shaded(Ink::Live, 0, opacity(0.38));
    canvas_of(app)
        .shown
        .grounds
        .iter()
        .filter(|ground| ground.color == wash)
        .copied()
        .collect()
}

#[test]
fn selects_in_the_editor_by_dragging_the_pointer_and_not_the_conversation() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", "alpha beta\ngamma")]);
    let advance = app.layout(Instant::now()).advance();

    let at = on_code(&mut app, 0, 6);
    act(&mut app, at);
    act(&mut app, WindowAction::Press);
    let at = on_code(&mut app, 1, 3);
    act(&mut app, at);
    let while_held = selected_rows(&mut app);
    act(&mut app, WindowAction::Release);
    let at = on_code(&mut app, 1, 5);
    act(&mut app, at);

    assert_eq!(while_held.len(), 2);
    assert!((while_held[0].width - advance * 5.0).abs() < 0.5);
    assert!((while_held[1].width - advance * 3.0).abs() < 0.5);
    let after_release = selected_rows(&mut app);
    assert_eq!(after_release, while_held);
    assert!(!app.window.pressing);
    assert!(!app.sessions.active().conversation.is_dragging());
}

#[test]
fn takes_a_word_on_a_double_click_in_the_editor() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", "alpha beta\ngamma")]);
    let advance = app.layout(Instant::now()).advance();

    for _ in 0..2 {
        let at = on_code(&mut app, 0, 7);
        act(&mut app, at);
        act(&mut app, WindowAction::Press);
        act(&mut app, WindowAction::Release);
    }

    let selected = selected_rows(&mut app);
    assert_eq!(selected.len(), 1);
    assert!((selected[0].width - advance * 4.0).abs() < 0.5);
}

#[test]
fn keeps_the_renamed_session_in_the_bar_of_the_editor() {
    let (mut app, _receiver, _) = opened(&[("src/a.rs", FILE)]);

    app.sessions
        .active_mut()
        .identity
        .rename("refonte", "Dot", Instant::now());
    crate::app::tests::settled(&mut app);

    assert!(
        shown_strip(&app).contains("refonte"),
        "{}",
        shown_strip(&app)
    );
}
