mod gpu;
mod layout;
mod mark;
mod scene;

use winit::dpi::PhysicalSize;

use crate::error::DesktopError;

pub(crate) use self::gpu::GpuCanvas;
pub(crate) use self::layout::{Fit, Layout, TextSize};
pub(crate) use self::mark::{Gauge, Icon, Paint, Rectangle, Reticle, Strip, Target};
pub(crate) use self::scene::{Bounds, Pane, Scene};

pub(crate) trait Canvas {
    fn resize(&mut self, size: PhysicalSize<u32>);

    fn show(&mut self, scene: Scene);

    fn point(&mut self, reticle: Option<Reticle>);

    fn allow_composition(&mut self, caret: Option<Bounds>);

    fn request_frame(&mut self);

    fn paint(&mut self) -> Result<(), DesktopError>;
}
