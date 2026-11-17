mod gpu;
mod icon;
mod layout;
mod line;
mod strip;

use winit::dpi::PhysicalSize;

use crate::error::DesktopError;

pub(crate) use self::gpu::GpuCanvas;
pub(crate) use self::icon::Icon;
pub(crate) use self::layout::Layout;
pub(crate) use self::line::{Ink, Line, Span};
pub(crate) use self::strip::{Gauge, Strip};

pub(crate) trait Canvas {
    fn resize(&mut self, size: PhysicalSize<u32>);

    fn show(&mut self, lines: Vec<Line>, strip: Option<Strip>);

    fn paint(&mut self) -> Result<(), DesktopError>;
}
