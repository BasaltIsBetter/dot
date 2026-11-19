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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(Arc::clone(&window))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&device_descriptor()))?;

        let size = window.inner_size();
        let configuration = surface
            .get_default_config(&adapter, size.width, size.height)
            .map(with_one_frame_in_flight)
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
        text.prepare(&self.device, &self.queue, &runs, layout, resolution)?;
        Ok(true)
    }

    fn prepare_shapes(&mut self) -> bool {
        let layout = &self.scene.layout;
        let strip = self.scene.strip.as_ref();
        if strip.is_none() && self.reticle.is_none() {
            return false;
        }

        let bottom = strip.map_or(layout.height(), |_| layout.strip_top());
        let hairlines = self
            .reticle
            .iter()
            .flat_map(|reticle| reticle.hairlines(layout.width(), bottom));
        let marks = strip.into_iter().flat_map(|strip| strip.rectangles(layout));
        let rings = self.reticle.as_ref().map(Reticle::ring);
        let shapes = self
            .shapes
            .get_or_insert_with(|| ShapeLayer::new(&self.device, self.configuration.format));
        shapes.prepare(
            &self.device,
            &self.queue,
            hairlines.chain(marks),
            rings,
            [layout.width(), layout.height()],
        );
        true
    }
}

impl Canvas for GpuCanvas {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.configuration.width = size.width;
        self.configuration.height = size.height;
        self.configure();
        self.window.request_redraw();
    }

    fn show(&mut self, scene: Scene) {
        if self.scene != scene {
            self.scene = scene;
            self.window.request_redraw();
        }
    }

    fn point(&mut self, reticle: Option<Reticle>) {
        if self.reticle != reticle {
            self.reticle = reticle;
            self.window.request_redraw();
        }
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

    fn paint(&mut self) -> Result<(), DesktopError> {
        let frame = match acquire(self.surface.get_current_texture())? {
            AcquiredFrame::Ready(frame) => frame,
            AcquiredFrame::Skipped => return Ok(()),
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

fn with_one_frame_in_flight(
    configuration: wgpu::SurfaceConfiguration,
) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        desired_maximum_frame_latency: FRAMES_IN_FLIGHT,
        ..configuration
    }
}

fn device_descriptor() -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/gpu/mod.rs"]
mod tests;
