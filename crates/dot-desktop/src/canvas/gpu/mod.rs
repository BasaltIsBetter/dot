mod frame;
mod shape;
mod text;

use std::sync::Arc;

use glyphon::Resolution;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use self::frame::{AcquiredFrame, acquire, encode_frame};
use self::shape::ShapeLayer;
use self::text::TextLayer;
use crate::canvas::Canvas;
use crate::canvas::layout::Layout;
use crate::canvas::line::Line;
use crate::canvas::strip::Strip;
use crate::error::DesktopError;

const FRAMES_IN_FLIGHT: u32 = 1;

pub(crate) struct GpuCanvas {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: wgpu::SurfaceConfiguration,
    lines: Vec<Line>,
    strip: Option<Strip>,
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
            lines: Vec::new(),
            strip: None,
            text: None,
            shapes: None,
        };
        canvas.configure();
        Ok(canvas)
    }

    fn configure(&self) {
        self.surface.configure(&self.device, &self.configuration);
    }

    fn layout(&self) -> Layout {
        let size = PhysicalSize::new(self.configuration.width, self.configuration.height);
        Layout::new(size, self.window.scale_factor())
    }

    fn prepare_text(&mut self, layout: Layout) -> Result<bool, DesktopError> {
        let strip_text = self.strip.as_ref().map(|strip| &strip.text);
        if self.lines.is_empty() && strip_text.is_none() {
            return Ok(false);
        }

        let resolution = Resolution {
            width: self.configuration.width,
            height: self.configuration.height,
        };
        let text = self.text.get_or_insert_with(|| {
            TextLayer::new(&self.device, &self.queue, self.configuration.format)
        });
        text.prepare(
            &self.device,
            &self.queue,
            &self.lines,
            strip_text,
            layout,
            resolution,
        )?;
        Ok(true)
    }

    fn prepare_shapes(&mut self, layout: &Layout) -> bool {
        let Some(strip) = self.strip.as_ref() else {
            return false;
        };

        let rectangles = strip.rectangles(layout);
        let shapes = self
            .shapes
            .get_or_insert_with(|| ShapeLayer::new(&self.device, self.configuration.format));
        shapes.prepare(
            &self.device,
            &self.queue,
            &rectangles,
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

    fn show(&mut self, lines: Vec<Line>, strip: Option<Strip>) {
        if self.lines != lines || self.strip != strip {
            self.lines = lines;
            self.strip = strip;
            self.window.request_redraw();
        }
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
        let layout = self.layout();
        let has_text = self.prepare_text(layout)?;
        let has_shapes = self.prepare_shapes(&layout);
        let text = self.text.as_ref().filter(|_| has_text);
        let shapes = self.shapes.as_ref().filter(|_| has_shapes);
        let commands = encode_frame(&self.device, &target, shapes, text)?;
        self.queue.submit([commands]);
        self.queue.present(frame);

        if let Some(text) = self.text.as_mut() {
            text.trim();
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
