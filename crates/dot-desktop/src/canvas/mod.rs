mod gpu;
mod layout;
mod line;
mod text;

use winit::dpi::PhysicalSize;

use crate::error::DesktopError;

pub(crate) use self::gpu::GpuCanvas;
pub(crate) use self::layout::Layout;
pub(crate) use self::line::{Ink, Line, Span};

pub(crate) trait Canvas {
    fn resize(&mut self, size: PhysicalSize<u32>);

    fn show(&mut self, lines: Vec<Line>);

    fn paint(&mut self) -> Result<(), DesktopError>;
}
