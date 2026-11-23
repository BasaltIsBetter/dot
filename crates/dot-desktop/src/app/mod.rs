mod answer;
mod frame;
mod handler;
mod input;

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Instant;

use dot_api::{Chat, Connector, PermissionMode, Session, SessionStatus, Translator, Workspace};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::ModifiersState;

use crate::app::input::Editing;
use crate::canvas::{Canvas, GpuCanvas, Layout};
use crate::editor::DiskNews;
use crate::error::DesktopError;
use crate::event::{Delivery, Flow, WindowAction};
use crate::reply::ReplyNews;
use crate::sessions::{SessionKey, Sessions};
use crate::sign_in::{Report, SignIn};
use crate::status_bar::StatusBar;
use crate::worker::Mailbox;

pub(crate) trait Parts {
    type Translator: Translator;
    type Connector: Connector + 'static;
    type Session: Session + 'static;
    type Chat: Chat + 'static;
    type Workspace: Workspace + 'static;
    type Mailbox: Mailbox<Report>
        + Mailbox<SessionStatus>
        + Mailbox<(SessionKey, ReplyNews)>
        + Mailbox<DiskNews>;
    type Canvas: Canvas;
}

type Chosen<T, K, S, H, W> = fn() -> (T, K, S, H, W);

pub(crate) struct OnMac<T, K, S, H, W>(PhantomData<Chosen<T, K, S, H, W>>);

pub(crate) type Opener<P> = Box<dyn FnMut() -> (<P as Parts>::Session, <P as Parts>::Chat)>;

pub(crate) struct DesktopApp<'a, P: Parts> {
    translator: &'a P::Translator,
    sign_in: SignIn<P::Connector, P::Mailbox>,
    status_bar: StatusBar<P::Mailbox>,
    sessions: Sessions<P>,
    editing: Editing<P::Workspace, P::Mailbox>,
    open: Opener<P>,
    connector: Arc<P::Connector>,
    mode: PermissionMode,
    mailbox: P::Mailbox,
    screen: Screen,
    canvas: Option<P::Canvas>,
    window: WindowState,
    wake_at: Option<Instant>,
    failure: Option<DesktopError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    SignIn,
    Chat,
}

#[derive(Debug, Default)]
struct WindowState {
    size: PhysicalSize<u32>,
    scale_factor: f64,
    cursor: Option<PhysicalPosition<f64>>,
    focused: bool,
    modifiers: ModifiersState,
}

impl<T, K, S, H, W> Parts for OnMac<T, K, S, H, W>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
    W: Workspace + 'static,
{
    type Translator = T;
    type Connector = K;
    type Session = S;
    type Chat = H;
    type Workspace = W;
    type Mailbox = Arc<EventLoopProxy<Delivery>>;
    type Canvas = GpuCanvas;
}

impl<'a, P: Parts> DesktopApp<'a, P> {
    pub(crate) fn new(
        translator: &'a P::Translator,
        connector: P::Connector,
        mut open: Opener<P>,
        mode: PermissionMode,
        (mailbox, workspace): (P::Mailbox, Arc<P::Workspace>),
    ) -> Self {
        let connector = Arc::new(connector);
        let sessions = Sessions::new(open(), &connector, mode, &mailbox);
        Self {
            translator,
            sign_in: SignIn::new(Arc::clone(&connector), mailbox.clone()),
            status_bar: StatusBar::new(mailbox.clone()),
            editing: Editing::new(workspace, mailbox.clone()),
            sessions,
            open,
            connector,
            mode,
            mailbox,
            screen: Screen::SignIn,
            canvas: None,
            window: WindowState::default(),
            wake_at: None,
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
            Delivery::Reply(key, news) => self.hear(key, news, now),
            Delivery::Disk(news) => self.disk_news(news, now),
        }
        self.request_frame();
    }

    fn enter_chat_once_connected(&mut self) {
        if self.screen == Screen::SignIn && self.sign_in.connection().is_some() {
            self.screen = Screen::Chat;
        }
    }

    fn open_session(&mut self, now: Instant) {
        let parts = (self.open)();
        self.sessions
            .open(parts, &self.connector, (self.mode, &self.mailbox), now);
    }

    fn refresh_status(&mut self) {
        let session = Arc::clone(&self.sessions.active().session);
        self.status_bar.refresh(session);
    }

    fn request_frame(&mut self) {
        if let Some(canvas) = self.canvas.as_mut() {
            canvas.request_frame();
        }
    }

    fn layout(&self) -> Layout {
        let window = &self.window;
        match self.screen {
            Screen::SignIn => Layout::zoomed(window.size, window.scale_factor),
            Screen::Chat => Layout::reflowed(window.size, window.scale_factor),
        }
    }

    fn stop_on(&mut self, error: DesktopError) -> Flow {
        self.failure = Some(error);
        Flow::Exit
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/mod.rs"]
pub(crate) mod tests;
