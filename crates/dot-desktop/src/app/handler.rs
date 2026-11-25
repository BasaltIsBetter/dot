use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::WindowId;

use crate::app::{DesktopApp, Parts};
use crate::canvas::GpuCanvas;
use crate::error::DesktopError;
use crate::event::{Delivery, WindowAction, follow, window_action};
use crate::shell::{open_main_window, press_moves};

impl<P: Parts<Canvas = GpuCanvas>> DesktopApp<'_, P> {
    fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), DesktopError> {
        let window = open_main_window(event_loop, self.translator)?;
        let canvas = GpuCanvas::from_window(Arc::clone(&window))?;
        window.set_visible(true);

        self.window.size = window.inner_size();
        self.window.scale_factor = window.scale_factor();
        self.window.focused = window.has_focus();
        self.canvas = Some(canvas);
        self.request_frame();
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
                self.refresh_status();
            }
            Err(error) => follow(event_loop, self.stop_on(error)),
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.request_frame();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let flow = self
            .wake_at
            .filter(|_| !self.window.hidden)
            .map_or(ControlFlow::Wait, ControlFlow::WaitUntil);
        event_loop.set_control_flow(flow);
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
        let cursor = self.window.cursor;
        let moves = action == WindowAction::Press
            && self
                .canvas
                .as_ref()
                .is_some_and(|canvas| press_moves(canvas.window(), cursor));
        if moves {
            return;
        }
        follow(event_loop, self.handle(action, Instant::now()));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.sign_in.stop();
        self.status_bar.stop();
        self.keeper.stop();
        self.sessions.stop_all();
        self.editing.finish();
    }
}
