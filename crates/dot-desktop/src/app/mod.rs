mod answer;
mod frame;
mod handler;
mod input;
mod settings;

use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use dot_api::{
    Chat, Connector, Preferences, Session, SessionStatus, Settings, Translator, Workspace,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::ModifiersState;

use crate::Choices;
use crate::app::input::Editing;
use crate::canvas::{Canvas, GpuCanvas, Layout, Strip};
use crate::editor::DiskNews;
use crate::error::DesktopError;
use crate::event::{Delivery, Flow, WindowAction};
use crate::reply::ReplyNews;
use crate::sessions::{NameField, SessionKey, Sessions};
use crate::settings::{Keeper, SettingsNews, SettingsScreen};
use crate::shell::ChosenFolder;
use crate::sign_in::{Report, SignIn};
use crate::status_bar::StatusBar;
use crate::worker::Mailbox;
use dot_transcript::{Glide, to_f32};

pub(crate) trait Parts {
    type Translator: Translator;
    type Connector: Connector + 'static;
    type Session: Session + 'static;
    type Chat: Chat + 'static;
    type Workspace: Workspace + 'static;
    type Preferences: Preferences + 'static;
    type Mailbox: Mailbox<Report>
        + Mailbox<SessionStatus>
        + Mailbox<(SessionKey, ReplyNews)>
        + Mailbox<DiskNews>
        + Mailbox<SettingsNews>
        + Mailbox<ChosenFolder>;
    type Canvas: Canvas;
}

type Chosen<T, K, S, H, W, F> = fn() -> (T, K, S, H, W, F);

pub(crate) struct OnMac<T, K, S, H, W, F>(PhantomData<Chosen<T, K, S, H, W, F>>);

pub(crate) type Opener<P> =
    Box<dyn FnMut(Option<PathBuf>) -> (<P as Parts>::Session, <P as Parts>::Chat)>;

pub(crate) struct DesktopApp<'a, P: Parts> {
    translator: &'a P::Translator,
    sign_in: SignIn<P::Connector, P::Mailbox>,
    status_bar: StatusBar<P::Mailbox>,
    sessions: Sessions<P>,
    editing: Editing<P::Workspace, P::Mailbox>,
    open: Opener<P>,
    connector: Arc<P::Connector>,
    settings: Settings,
    keeper: Keeper<P::Preferences, P::Mailbox>,
    settings_screen: SettingsScreen,
    model_tuned_at: Option<Instant>,
    effort_tuned_at: Option<Instant>,
    reading: Glide,
    strip: Option<Strip>,
    renaming: Option<NameField>,
    launch_folder: Option<PathBuf>,
    key_check: Option<String>,
    key_shown: Option<String>,
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
    hidden: bool,
    modifiers: ModifiersState,
    pressing: bool,
    last_press: Option<(Instant, [f32; 2], u8)>,
}

impl<T, K, S, H, W, F> Parts for OnMac<T, K, S, H, W, F>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
    W: Workspace + 'static,
    F: Preferences + 'static,
{
    type Translator = T;
    type Connector = K;
    type Session = S;
    type Chat = H;
    type Workspace = W;
    type Preferences = F;
    type Mailbox = Arc<EventLoopProxy<Delivery>>;
    type Canvas = GpuCanvas;
}

impl<'a, P: Parts> DesktopApp<'a, P> {
    pub(crate) fn new(
        translator: &'a P::Translator,
        connector: P::Connector,
        mut open: Opener<P>,
        choices: Choices<P::Preferences>,
        (mailbox, workspace): (P::Mailbox, Arc<P::Workspace>),
    ) -> Self {
        let connector = Arc::new(connector);
        let Choices {
            preferences,
            settings,
            defaults,
            folder,
            seed,
        } = choices;
        let answering = (settings.permissions, settings.tuning);
        let mut sessions = Sessions::new(
            open(folder.clone()),
            &connector,
            (answering, &mailbox),
            (settings.color, seed),
        );
        sessions.active_mut().folder.clone_from(&folder);
        Self {
            translator,
            sign_in: SignIn::new(Arc::clone(&connector), mailbox.clone()),
            status_bar: StatusBar::new(mailbox.clone()),
            editing: Editing::new(workspace, mailbox.clone()),
            sessions,
            open,
            connector,
            settings,
            keeper: Keeper::new(preferences, defaults, mailbox.clone()),
            settings_screen: SettingsScreen::new(),
            model_tuned_at: None,
            effort_tuned_at: None,
            reading: settings::reading_glide(&settings),
            strip: None,
            renaming: None,
            launch_folder: folder,
            key_check: None,
            key_shown: None,
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

    fn paint(&mut self, now: Instant) -> Result<(), DesktopError> {
        self.compose(now);
        self.canvas.as_mut().map_or(Ok(()), Canvas::paint)
    }

    fn handle(&mut self, action: WindowAction, now: Instant) -> Flow {
        let painted = match action {
            WindowAction::Close => return Flow::Exit,
            WindowAction::Paint if self.window.hidden => Ok(()),
            WindowAction::Paint => self.paint(now),
            WindowAction::Hide => {
                self.window.hidden = true;
                Ok(())
            }
            WindowAction::Show => {
                self.window.hidden = false;
                self.paint(now)
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
                self.connection_news(&report, now);
                self.sign_in.report(report);
                self.enter_chat_once_connected();
            }
            Delivery::Folder(ChosenFolder(folder)) => self.open_session_in(Some(folder), now),
            Delivery::Session(status) => self.status_bar.report(status),
            Delivery::Reply(key, news) => self.hear(key, news, now),
            Delivery::Disk(news) => self.disk_news(news, now),
            Delivery::Settings(news) => self.settings_news(&news, now),
        }
        self.request_frame();
    }

    fn enter_chat_once_connected(&mut self) {
        if self.screen == Screen::SignIn && self.sign_in.connection().is_some() {
            self.screen = Screen::Chat;
        }
    }

    fn open_session(&mut self, now: Instant) {
        let folder = self.sessions.active().folder.clone();
        self.open_session_in(folder, now);
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

    fn layout(&self, now: Instant) -> Layout {
        let window = &self.window;
        match self.screen {
            Screen::SignIn => Layout::zoomed_to(window.size, window.scale_factor, self.size()),
            Screen::Chat => Layout::fitted(window.size, window.scale_factor, self.fit(now)),
        }
    }

    fn pointer(&self) -> Option<[f32; 2]> {
        self.window
            .cursor
            .map(|cursor| [to_f32(cursor.x), to_f32(cursor.y)])
    }

    fn stop_on(&mut self, error: DesktopError) -> Flow {
        self.failure = Some(error);
        Flow::Exit
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/app/mod.rs"]
pub(crate) mod tests;
