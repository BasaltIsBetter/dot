use std::path::PathBuf;

use dot_api::SessionStatus;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, ModifiersState, SmolStr};

use crate::editor::DiskNews;
use crate::keeping::KeptNews;
use crate::reply::ReplyNews;
use crate::sessions::SessionKey;
use crate::settings::SettingsNews;
use crate::shell::{ChosenFolder, native_keystroke, native_modifiers, quits};
use crate::sign_in::Report;

#[derive(Debug, PartialEq)]
pub(crate) enum WindowAction {
    Close,
    Resize(PhysicalSize<u32>),
    Rescale(f64),
    Paint,
    Hide,
    Show,
    Focus,
    Blur,
    Leave,
    Modifiers(ModifiersState),
    Key(Keystroke),
    Commit(String),
    Compose(String),
    Point(PhysicalPosition<f64>),
    Press,
    Release,
    Scroll(Wheel),
    Drop(PathBuf),
    Ignore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Keystroke {
    pub(crate) key: Key,
    pub(crate) text: Option<SmolStr>,
    pub(crate) modifiers: ModifiersState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Wheel {
    Lines(f32),
    Pixels(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Exit,
}

#[derive(Debug)]
pub(crate) enum Delivery {
    SignIn(Report),
    Session(SessionStatus),
    Reply(SessionKey, ReplyNews),
    Disk(DiskNews),
    Settings(SettingsNews),
    Folder(ChosenFolder),
    Kept(KeptNews),
}

impl From<KeptNews> for Delivery {
    fn from(news: KeptNews) -> Self {
        Self::Kept(news)
    }
}

impl From<ChosenFolder> for Delivery {
    fn from(chosen: ChosenFolder) -> Self {
        Self::Folder(chosen)
    }
}

impl From<SettingsNews> for Delivery {
    fn from(news: SettingsNews) -> Self {
        Self::Settings(news)
    }
}

impl From<DiskNews> for Delivery {
    fn from(news: DiskNews) -> Self {
        Self::Disk(news)
    }
}

impl From<Report> for Delivery {
    fn from(report: Report) -> Self {
        Self::SignIn(report)
    }
}

impl From<(SessionKey, ReplyNews)> for Delivery {
    fn from((key, news): (SessionKey, ReplyNews)) -> Self {
        Self::Reply(key, news)
    }
}

impl From<SessionStatus> for Delivery {
    fn from(status: SessionStatus) -> Self {
        Self::Session(status)
    }
}

pub(crate) fn window_action(event: &WindowEvent, modifiers: ModifiersState) -> WindowAction {
    match event {
        WindowEvent::CloseRequested => WindowAction::Close,
        WindowEvent::Resized(size) => WindowAction::Resize(*size),
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            WindowAction::Rescale(*scale_factor)
        }
        WindowEvent::RedrawRequested => WindowAction::Paint,
        WindowEvent::Occluded(true) => WindowAction::Hide,
        WindowEvent::Occluded(false) => WindowAction::Show,
        WindowEvent::Focused(true) => WindowAction::Focus,
        WindowEvent::Focused(false) => WindowAction::Blur,
        WindowEvent::ModifiersChanged(modifiers) => {
            WindowAction::Modifiers(native_modifiers(modifiers.state()))
        }
        WindowEvent::KeyboardInput { event, .. } => key_action(event, modifiers),
        WindowEvent::Ime(Ime::Commit(text)) => WindowAction::Commit(text.clone()),
        WindowEvent::Ime(Ime::Preedit(text, _)) => WindowAction::Compose(text.clone()),
        WindowEvent::Ime(Ime::Disabled) => WindowAction::Compose(String::new()),
        WindowEvent::CursorMoved { position, .. } => WindowAction::Point(*position),
        WindowEvent::CursorLeft { .. } => WindowAction::Leave,
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } => WindowAction::Press,
        WindowEvent::MouseInput {
            state: ElementState::Released,
            button: MouseButton::Left,
            ..
        } => WindowAction::Release,
        WindowEvent::MouseWheel { delta, .. } => WindowAction::Scroll(wheel(*delta)),
        WindowEvent::DroppedFile(path) => WindowAction::Drop(path.clone()),
        _ => WindowAction::Ignore,
    }
}

pub(crate) fn follow(event_loop: &ActiveEventLoop, flow: Flow) {
    if flow == Flow::Exit {
        event_loop.exit();
    }
}

fn key_action(event: &KeyEvent, modifiers: ModifiersState) -> WindowAction {
    if event.state != ElementState::Pressed {
        return WindowAction::Ignore;
    }
    let keystroke = native_keystroke(Keystroke {
        key: event.logical_key.clone(),
        text: event.text.clone(),
        modifiers,
    });
    if quits(&keystroke) {
        return WindowAction::Close;
    }
    WindowAction::Key(keystroke)
}

const fn wheel(delta: MouseScrollDelta) -> Wheel {
    match delta {
        MouseScrollDelta::LineDelta(_, rows) => Wheel::Lines(rows),
        MouseScrollDelta::PixelDelta(position) => Wheel::Pixels(position.y),
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-desktop/event.rs"]
mod tests;
