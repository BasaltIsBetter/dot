use std::marker::PhantomData;
use std::sync::Arc;

use dot_api::{Chat, Connector, Session, SessionStatus, Translator};
use winit::event_loop::EventLoopProxy;

use crate::canvas::{Canvas, GpuCanvas};
use crate::event::Delivery;
use crate::reply::ReplyNews;
use crate::sign_in::Report;
use crate::worker::Mailbox;

pub(crate) trait Parts {
    type Translator: Translator;
    type Connector: Connector + 'static;
    type Session: Session + 'static;
    type Chat: Chat + 'static;
    type Mailbox: Mailbox<Report> + Mailbox<SessionStatus> + Mailbox<ReplyNews>;
    type Canvas: Canvas;
}

type Chosen<T, K, S, H> = fn() -> (T, K, S, H);

pub(crate) struct OnMac<T, K, S, H>(PhantomData<Chosen<T, K, S, H>>);

impl<T, K, S, H> Parts for OnMac<T, K, S, H>
where
    T: Translator,
    K: Connector + 'static,
    S: Session + 'static,
    H: Chat + 'static,
{
    type Translator = T;
    type Connector = K;
    type Session = S;
    type Chat = H;
    type Mailbox = Arc<EventLoopProxy<Delivery>>;
    type Canvas = GpuCanvas;
}
