mod answer;
mod input;

use std::sync::Arc;
use std::time::Instant;

use dot_api::{Chat, Connector, PermissionMode, Session, SessionStatus, Translator};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::canvas::{Canvas, GpuCanvas, Layout, Scene, Strip};
use crate::conversation::Conversation;
use crate::error::DesktopError;
use crate::event::{Delivery, Flow, WindowAction, follow, window_action};
use crate::reply::{Replies, ReplyNews};
use crate::shell::open_main_window;
use crate::sign_in::{Report, SignIn};
use crate::status_bar::StatusBar;
use crate::worker::Mailbox;

pub(crate) struct DesktopApp<'a, T, K, S, H, M = EventLoopProxy<Delivery>, C = GpuCanvas> {
    translator: &'a T,
    sign_in: SignIn<K, M>,
    status_bar: StatusBar<S, M>,
    replies: Replies<K, H, M>,
    conversation: Conversation,
    canvas: Option<C>,
    window: WindowState,
    failure: Option<DesktopError>,
}

#[derive(Debug, Default)]
struct WindowState {
    size: PhysicalSize<u32>,
    scale_factor: f64,
    cursor: Option<PhysicalPosition<f64>>,
    focused: bool,
    modifiers: ModifiersState,
}

impl<'a, T, K, S, H, M, C> DesktopApp<'a, T, K, S, H, M, C>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
    M: Mailbox<Report> + Mailbox<SessionStatus> + Mailbox<ReplyNews>,
    C: Canvas,
{
    pub(crate) fn new(
        translator: &'a T,
        connector: K,
        session: S,
        chat: H,
        mode: PermissionMode,
        mailbox: M,
    ) -> Self {
        let connector = Arc::new(connector);
        Self {
            translator,
            sign_in: SignIn::new(Arc::clone(&connector), mailbox.clone()),
            replies: Replies::new(connector, chat, mode, mailbox.clone()),
            status_bar: StatusBar::new(session, mailbox),
            conversation: Conversation::default(),
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
                self.refresh(now);
                self.canvas.as_mut().map_or(Ok(()), Canvas::paint)
            }
            WindowAction::Resize(size) => {
                self.window.size = size;
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.resize(size);
                }
                self.refresh(now);
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
            Delivery::SignIn(report) => self.sign_in.report(report),
            Delivery::Session(status) => self.status_bar.report(status),
            Delivery::Reply(news) => self.hear(news, now),
        }
        self.refresh(now);
    }

    fn refresh(&mut self, now: Instant) {
        let layout = self.layout();
        let mut scene = if self.is_connected() {
            let focused = self.window.focused;
            self.conversation.prepare(&layout, now, self.translator);
            self.conversation
                .scene(&layout, now, focused, self.translator)
        } else {
            Scene {
                lines: self.sign_in.lines(self.translator),
                ..Scene::default()
            }
        };
        scene.layout = layout;
        scene.strip = self.strip(&layout);
        let reticle = self.reticle();
        let caret = self
            .is_connected()
            .then(|| self.conversation.caret_area(&layout));
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.show(scene);
            canvas.point(reticle);
            canvas.allow_composition(caret);
        }
    }

    fn layout(&self) -> Layout {
        let window = &self.window;
        if self.is_connected() {
            return Layout::reflowed(window.size, window.scale_factor);
        }
        Layout::zoomed(window.size, window.scale_factor)
    }

    const fn is_connected(&self) -> bool {
        self.sign_in.connection().is_some()
    }

    fn strip(&self, layout: &Layout) -> Option<Strip> {
        self.sign_in.connection()?;
        self.status_bar
            .strip(self.translator, layout.strip_columns())
    }

    fn stop_on(&mut self, error: DesktopError) -> Flow {
        self.failure = Some(error);
        Flow::Exit
    }
}

impl<T, K, S, H> DesktopApp<'_, T, K, S, H>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
{
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

impl<T, K, S, H> ApplicationHandler<Delivery> for DesktopApp<'_, T, K, S, H>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
{
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
