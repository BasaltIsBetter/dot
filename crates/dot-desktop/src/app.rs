use std::sync::Arc;

use dot_api::{Connector, Session, SessionStatus, Translator};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::canvas::{Canvas, GpuCanvas, Layout, Strip};
use crate::error::DesktopError;
use crate::event::{Delivery, Flow, WindowAction, follow, window_action};
use crate::shell::{open_main_window, pasted_text};
use crate::sign_in::{Input, KeyIntent, Method, Report, SignIn};
use crate::status_bar::StatusBar;
use crate::worker::Mailbox;

pub(crate) struct DesktopApp<'a, T, K, S, M = EventLoopProxy<Delivery>, C = GpuCanvas> {
    translator: &'a T,
    sign_in: SignIn<K, M>,
    status_bar: StatusBar<S, M>,
    canvas: Option<C>,
    window: WindowState,
    failure: Option<DesktopError>,
}

#[derive(Debug, Default)]
struct WindowState {
    size: PhysicalSize<u32>,
    scale_factor: f64,
    cursor: Option<PhysicalPosition<f64>>,
    modifiers: ModifiersState,
}

impl<'a, T, K, S, M, C> DesktopApp<'a, T, K, S, M, C>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    M: Mailbox<Report> + Mailbox<SessionStatus>,
    C: Canvas,
{
    pub(crate) fn new(translator: &'a T, connector: K, session: S, mailbox: M) -> Self {
        Self {
            translator,
            sign_in: SignIn::new(connector, mailbox.clone()),
            status_bar: StatusBar::new(session, mailbox),
            canvas: None,
            window: WindowState::default(),
            failure: None,
        }
    }

    pub(crate) fn into_result(self) -> Result<(), DesktopError> {
        self.failure.map_or(Ok(()), Err)
    }

    fn handle(&mut self, action: WindowAction) -> Flow {
        let painted = match action {
            WindowAction::Close => return Flow::Exit,
            WindowAction::Paint => self.canvas.as_mut().map_or(Ok(()), Canvas::paint),
            WindowAction::Resize(size) => {
                self.window.size = size;
                if let Some(canvas) = self.canvas.as_mut() {
                    canvas.resize(size);
                }
                self.refresh();
                Ok(())
            }
            other => {
                self.follow_input(other);
                Ok(())
            }
        };

        match painted {
            Ok(()) => Flow::Continue,
            Err(error) => self.stop_on(error),
        }
    }

    fn follow_input(&mut self, action: WindowAction) {
        match action {
            WindowAction::Rescale(scale_factor) => self.window.scale_factor = scale_factor,
            WindowAction::Focus => {
                self.status_bar.refresh();
                return;
            }
            WindowAction::Modifiers(modifiers) => self.window.modifiers = modifiers,
            WindowAction::Key(KeyIntent::Input(input)) => self.sign_in.input(input),
            WindowAction::Key(KeyIntent::Paste) => {
                if let Some(text) = pasted_text() {
                    self.sign_in.input(Input::Paste(text));
                }
            }
            WindowAction::Point(position) => {
                self.window.cursor = Some(position);
                self.input_under_cursor(Input::Point);
            }
            WindowAction::Press => self.input_under_cursor(Input::Press),
            _ => return,
        }
        self.refresh();
    }

    fn input_under_cursor(&mut self, input: fn(Method) -> Input) {
        let layout = Layout::new(self.window.size, self.window.scale_factor);
        let method = self
            .window
            .cursor
            .and_then(|cursor| layout.line_at(cursor))
            .and_then(|row| self.sign_in.method_at(row));
        if let Some(method) = method {
            self.sign_in.input(input(method));
        }
    }

    fn deliver(&mut self, delivery: Delivery) {
        match delivery {
            Delivery::SignIn(report) => self.sign_in.report(report),
            Delivery::Session(status) => self.status_bar.report(status),
        }
        self.refresh();
    }

    fn refresh(&mut self) {
        let lines = self.sign_in.lines(self.translator);
        let strip = self.strip();
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.show(lines, strip);
        }
    }

    fn strip(&self) -> Option<Strip> {
        self.sign_in.connection()?;
        let columns = Layout::new(self.window.size, self.window.scale_factor).strip_columns();
        self.status_bar.strip(self.translator, columns)
    }

    fn stop_on(&mut self, error: DesktopError) -> Flow {
        self.failure = Some(error);
        Flow::Exit
    }
}

impl<T: Translator, K: Connector + 'static, S: Session + 'static> DesktopApp<'_, T, K, S> {
    fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), DesktopError> {
        let window = open_main_window(event_loop, self.translator)?;
        let canvas = GpuCanvas::from_window(Arc::clone(&window))?;
        window.set_visible(true);

        self.window.size = window.inner_size();
        self.window.scale_factor = window.scale_factor();
        self.canvas = Some(canvas);
        Ok(())
    }
}

impl<T, K, S> ApplicationHandler<Delivery> for DesktopApp<'_, T, K, S>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
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
        self.deliver(delivery);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let action = window_action(&event, self.window.modifiers);
        follow(event_loop, self.handle(action));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.sign_in.stop();
        self.status_bar.stop();
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-desktop/app.rs"]
mod tests;
