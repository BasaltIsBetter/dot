use std::sync::Arc;

use dot_api::{Connector, Text, Translator};
use objc2::MainThreadMarker;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::canvas::{Canvas, GpuCanvas, Layout};
use crate::error::DesktopError;
use crate::event::{WindowAction, window_action};
use crate::shell::{main_window_attributes, paint_background_black, pasted_text, translate_menu};
use crate::sign_in::{Input, KeyIntent, Method, Report, SignIn};
use crate::worker::Mailbox;

const OPENED_ON_THE_MAIN_THREAD: &str = "winit opens the window on the main thread";

pub(crate) struct DesktopApp<'a, T, K, M = EventLoopProxy<Report>, C = GpuCanvas> {
    translator: &'a T,
    sign_in: SignIn<K, M>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    Continue,
    Exit,
}

impl<'a, T, K, M, C> DesktopApp<'a, T, K, M, C>
where
    T: Translator,
    K: Connector + 'static,
    M: Mailbox<Report>,
    C: Canvas,
{
    pub(crate) fn new(translator: &'a T, connector: K, mailbox: M) -> Self {
        Self {
            translator,
            sign_in: SignIn::new(connector, mailbox),
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

    fn refresh(&mut self) {
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.show(self.sign_in.lines(self.translator));
        }
    }

    fn stop_on(&mut self, error: DesktopError) -> Flow {
        self.failure = Some(error);
        Flow::Exit
    }
}

impl<T: Translator, K: Connector + 'static> DesktopApp<'_, T, K> {
    fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), DesktopError> {
        let main_thread = MainThreadMarker::new().expect(OPENED_ON_THE_MAIN_THREAD);
        let title = self.translator.translate(Text::WindowTitle);
        let window = Arc::new(event_loop.create_window(main_window_attributes(&title))?);

        translate_menu(self.translator, main_thread);
        paint_background_black(&window, main_thread);
        let canvas = GpuCanvas::from_window(Arc::clone(&window))?;
        window.set_visible(true);

        self.window.size = window.inner_size();
        self.window.scale_factor = window.scale_factor();
        self.canvas = Some(canvas);
        Ok(())
    }
}

impl<T: Translator, K: Connector + 'static> ApplicationHandler<Report> for DesktopApp<'_, T, K> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.canvas.is_some() {
            return;
        }
        match self.open_window(event_loop) {
            Ok(()) => self.sign_in.start(),
            Err(error) => follow(event_loop, self.stop_on(error)),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, report: Report) {
        self.sign_in.report(report);
        self.refresh();
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
    }
}

fn follow(event_loop: &ActiveEventLoop, flow: Flow) {
    if flow == Flow::Exit {
        event_loop.exit();
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-desktop/app.rs"]
mod tests;
