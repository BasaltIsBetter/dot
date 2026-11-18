mod gpu;
mod layout;
mod line;
mod mark;
mod scene;

use winit::dpi::PhysicalSize;

use crate::error::DesktopError;

pub(crate) use self::gpu::GpuCanvas;
pub(crate) use self::layout::{Layout, to_f32};
pub(crate) use self::line::{Ink, Line, Span};
pub(crate) use self::mark::{Gauge, Icon, Rectangle, Reticle, Strip};
pub(crate) use self::scene::{Bounds, Pane, Placed, Scene};

pub(crate) trait Canvas {
    fn resize(&mut self, size: PhysicalSize<u32>);

    fn show(&mut self, scene: Scene);

    fn point(&mut self, reticle: Option<Reticle>);

    fn allow_composition(&mut self, caret: Option<Bounds>);

    fn paint(&mut self) -> Result<(), DesktopError>;
}
