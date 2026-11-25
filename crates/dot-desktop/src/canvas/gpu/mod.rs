mod frame;
mod shape;
mod text;

use std::sync::Arc;

use glyphon::Resolution;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

use self::frame::{AcquiredFrame, acquire, encode_frame};
use self::shape::ShapeLayer;
use self::text::TextLayer;
use crate::canvas::Canvas;
use crate::canvas::mark::Reticle;
use crate::canvas::scene::{Bounds, Scene};
use crate::error::DesktopError;

const FRAMES_IN_FLIGHT: u32 = 1;
const SMALLEST_SIDE: u32 = 1;

pub(crate) struct GpuCanvas {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: wgpu::SurfaceConfiguration,
    scene: Scene,
    reticle: Option<Reticle>,
    composition: Option<Bounds>,
    text: Option<TextLayer>,
    shapes: Option<ShapeLayer>,
}

impl GpuCanvas {
    pub(crate) fn from_window(window: Arc<Window>) -> Result<Self, DesktopError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(Arc::clone(&window)),
        ));
        let surface = instance.create_surface(Arc::clone(&window))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let card = adapter.get_info();
        tracing::info!(card = %card.name, through = %card.backend, "Dot draws");
        let required_limits = wgpu::Limits {
            max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d,
            ..wgpu::Limits::default()
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_limits,
                ..wgpu::DeviceDescriptor::default()
            }))?;
        device.on_uncaptured_error(Arc::new(|error: wgpu::Error| {
            tracing::error!(%error, "the GPU refused what Dot asked of it");
        }));

        let largest = device.limits().max_texture_dimension_2d;
        let (width, height) = drawable(window.inner_size(), largest);
        let configuration = surface
            .get_default_config(&adapter, width, height)
            .map(paced_by_the_screen)
            .ok_or(DesktopError::UnsupportedSurface)?;

        let canvas = Self {
            window,
            surface,
            device,
            queue,
            configuration,
            scene: Scene::default(),
            reticle: None,
            composition: None,
            text: None,
            shapes: None,
        };
        canvas.configure();
        Ok(canvas)
    }

    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    fn configure(&self) {
        self.surface.configure(&self.device, &self.configuration);
    }

    fn prepare_text(&mut self) -> Result<bool, DesktopError> {
        let layout = &self.scene.layout;
        let runs = self.scene.runs();
        if runs.is_empty() {
            return Ok(false);
        }

        let resolution = Resolution {
            width: self.configuration.width,
            height: self.configuration.height,
        };
        let text = self.text.get_or_insert_with(|| {
            TextLayer::new(&self.device, &self.queue, self.configuration.format)
        });
        let palette = self.scene.palette;
        text.prepare(
            &self.device,
            &self.queue,
            &runs,
            (layout, palette),
            resolution,
        )?;
        Ok(true)
    }

    fn prepare_shapes(&mut self) -> bool {
        let scene = &self.scene;
        let layout = &scene.layout;
        let strip = scene.strip.as_ref();
        let drawn_by_the_scene =
            !(scene.grounds.is_empty() && scene.marks.is_empty() && scene.veils.is_empty());
        if strip.is_none() && self.reticle.is_none() && !drawn_by_the_scene {
            return false;
        }

        let bottom = strip.map_or(layout.height(), |_| layout.strip_top());
        let hairlines = self
            .reticle
            .iter()
            .flat_map(|reticle| reticle.hairlines(layout.width(), bottom));
        let strip_marks = strip.into_iter().flat_map(|strip| strip.rectangles(layout));
        let frames = self.reticle.iter().flat_map(Reticle::frame);
        let under = scene
            .grounds
            .iter()
            .copied()
            .chain(hairlines)
            .chain(scene.marks.iter().copied())
            .chain(strip_marks)
            .chain(frames);
        let shapes = self
            .shapes
            .get_or_insert_with(|| ShapeLayer::new(&self.device, self.configuration.format));
        shapes.prepare(
            &self.device,
            &self.queue,
            under,
            scene.veils.iter().copied(),
            ([layout.width(), layout.height()], scene.palette),
        );
        true
    }
}

impl Canvas for GpuCanvas {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        let largest = self.device.limits().max_texture_dimension_2d;
        let (width, height) = drawable(size, largest);
        if (width, height) == (self.configuration.width, self.configuration.height) {
            return;
        }
        self.configuration.width = width;
        self.configuration.height = height;
        self.configure();
    }

    fn show(&mut self, scene: Scene) {
        self.scene = scene;
    }

    fn point(&mut self, reticle: Option<Reticle>) {
        self.reticle = reticle;
    }

    fn allow_composition(&mut self, caret: Option<Bounds>) {
        if self.composition == caret {
            return;
        }
        if self.composition.is_some() != caret.is_some() {
            self.window.set_ime_allowed(caret.is_some());
        }
        if let Some(area) = caret {
            self.window.set_ime_cursor_area(
                PhysicalPosition::new(area.left, area.top),
                PhysicalSize::new(area.width, area.height),
            );
        }
        self.composition = caret;
    }

    fn request_frame(&mut self) {
        self.window.request_redraw();
    }

    fn paint(&mut self) -> Result<(), DesktopError> {
        let frame = match acquire(self.surface.get_current_texture())? {
            AcquiredFrame::Ready(frame) => frame,
            AcquiredFrame::Skipped => return Ok(()),
            AcquiredFrame::Late => {
                self.window.request_redraw();
                return Ok(());
            }
            AcquiredFrame::Outdated => {
                self.configure();
                self.window.request_redraw();
                return Ok(());
            }
        };

        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let has_text = self.prepare_text()?;
        let has_shapes = self.prepare_shapes();
        let text = self.text.as_ref().filter(|_| has_text);
        let shapes = self.shapes.as_ref().filter(|_| has_shapes);
        let commands = encode_frame(&self.device, &target, shapes, text)?;
        self.queue.submit([commands]);
        self.window.pre_present_notify();
        self.queue.present(frame);

        if let Some(text) = self.text.as_mut() {
            text.trim();
        }
        if self.scene.animating {
            self.window.request_redraw();
        }
        Ok(())
    }
}

fn drawable(size: PhysicalSize<u32>, largest: u32) -> (u32, u32) {
    let bounded = |pixels: u32| pixels.clamp(SMALLEST_SIDE, largest.max(SMALLEST_SIDE));
    (bounded(size.width), bounded(size.height))
}

fn paced_by_the_screen(configuration: wgpu::SurfaceConfiguration) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        desired_maximum_frame_latency: FRAMES_IN_FLIGHT,
        present_mode: wgpu::PresentMode::Fifo,
        ..configuration
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/gpu/mod.rs"]
mod tests;
