mod answer;
mod input;
mod parts;

use std::sync::Arc;
use std::time::Instant;

use dot_api::PermissionMode;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::canvas::{Canvas, GpuCanvas, Layout, Scene};
use crate::conversation::Conversation;
use crate::error::DesktopError;
use crate::event::{Delivery, Flow, WindowAction, follow, window_action};
use crate::reply::Replies;
use crate::shell::open_main_window;
use crate::sign_in::SignIn;
use crate::status_bar::StatusBar;

pub(crate) use self::parts::{OnMac, Parts};

pub(crate) struct DesktopApp<'a, P: Parts> {
    translator: &'a P::Translator,
    sign_in: SignIn<P::Connector, P::Mailbox>,
    status_bar: StatusBar<P::Session, P::Mailbox>,
    replies: Replies<P::Connector, P::Session, P::Chat, P::Mailbox>,
    screen: Screen,
    canvas: Option<P::Canvas>,
    window: WindowState,
    failure: Option<DesktopError>,
}

enum Screen {
    SignIn,
    Chat(Box<Conversation>),
}

#[derive(Debug, Default)]
struct WindowState {
    size: PhysicalSize<u32>,
    scale_factor: f64,
    cursor: Option<PhysicalPosition<f64>>,
    focused: bool,
    modifiers: ModifiersState,
}

impl<'a, P: Parts> DesktopApp<'a, P> {
    pub(crate) fn new(
        translator: &'a P::Translator,
        connector: P::Connector,
        session: P::Session,
        chat: P::Chat,
        mode: PermissionMode,
        mailbox: P::Mailbox,
    ) -> Self {
        let connector = Arc::new(connector);
        let session = Arc::new(session);
        Self {
            translator,
            sign_in: SignIn::new(Arc::clone(&connector), mailbox.clone()),
            replies: Replies::new(connector, Arc::clone(&session), chat, mode, mailbox.clone()),
            status_bar: StatusBar::new(session, mailbox),
            screen: Screen::SignIn,
            canvas: None,
            window: WindowState::default(),
            failure: None,
        }
    }

    pub(crate) fn into_result(self) -> Result<(), DesktopError> {
        self.failure.map_or(Ok(()), Err)
    }

    fn handle(&mut self, action: WindowAction, now: Instant) -> Flow {
        let painted = match action {
            WindowAction::Close => return Flow::Exit,
            WindowAction::Paint => {
                self.compose(now);
                self.canvas.as_mut().map_or(Ok(()), Canvas::paint)
            }
            WindowAction::Resize(size) => {
                self.window.size = size;
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.resize(size);
                }
                self.request_frame();
                Ok(())
            }
            other => {
                self.follow_input(other, now);
                Ok(())
            }
        };

        match painted {
            Ok(()) => Flow::Continue,
            Err(error) => self.stop_on(error),
        }
    }

    fn deliver(&mut self, delivery: Delivery, now: Instant) {
        match delivery {
            Delivery::SignIn(report) => {
                self.sign_in.report(report);
                self.enter_chat_once_connected();
            }
            Delivery::Session(status) => self.status_bar.report(status),
            Delivery::Reply(news) => self.hear(news, now),
        }
        self.request_frame();
    }

    fn enter_chat_once_connected(&mut self) {
        if matches!(self.screen, Screen::SignIn) && self.sign_in.connection().is_some() {
            self.screen = Screen::Chat(Box::default());
        }
    }

    fn request_frame(&mut self) {
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.request_frame();
        }
    }

    fn compose(&mut self, now: Instant) {
        let layout = self.layout();
        let focused = self.window.focused;
        let reticle = self.reticle();
        let (mut scene, caret) = match &mut self.screen {
            Screen::SignIn => {
                let lines = self.sign_in.lines(self.translator);
                (
                    Scene {
                        lines,
                        ..Scene::default()
                    },
                    None,
                )
            }
            Screen::Chat(conversation) => {
                conversation.prepare(&layout, now, self.translator);
                let mut scene = conversation.scene(&layout, now, focused, self.translator);
                scene.strip = self
                    .status_bar
                    .strip(self.translator, layout.strip_columns());
                (scene, Some(conversation.caret_area(&layout)))
            }
        };
        scene.layout = layout;
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.show(scene);
            canvas.point(reticle);
            canvas.allow_composition(caret);
        }
    }

    fn layout(&self) -> Layout {
        let window = &self.window;
        match self.screen {
            Screen::SignIn => Layout::zoomed(window.size, window.scale_factor),
            Screen::Chat(_) => Layout::reflowed(window.size, window.scale_factor),
        }
    }

    fn stop_on(&mut self, error: DesktopError) -> Flow {
        self.failure = Some(error);
        Flow::Exit
    }
}

impl<P: Parts<Canvas = GpuCanvas>> DesktopApp<'_, P> {
    fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), DesktopError> {
        let window = open_main_window(event_loop, self.translator)?;
        let canvas = GpuCanvas::from_window(Arc::clone(&window))?;
        window.set_visible(true);

        self.window.size = window.inner_size();
        self.window.scale_factor = window.scale_factor();
        self.window.focused = window.has_focus();
        self.canvas = Some(canvas);
        Ok(())
    }
}

impl<P: Parts<Canvas = GpuCanvas>> ApplicationHandler<Delivery> for DesktopApp<'_, P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.canvas.is_some() {
            return;
        }
        match self.open_window(event_loop) {
            Ok(()) => {
                self.sign_in.start();
                self.status_bar.refresh();
            }
            Err(error) => follow(event_loop, self.stop_on(error)),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, delivery: Delivery) {
        self.deliver(delivery, Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let action = window_action(&event, self.window.modifiers);
        follow(event_loop, self.handle(action, Instant::now()));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.sign_in.stop();
        self.status_bar.stop();
        self.replies.stop_all();
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/mod.rs"]
pub(crate) mod tests;
