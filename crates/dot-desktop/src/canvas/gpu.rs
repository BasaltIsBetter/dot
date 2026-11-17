use std::sync::Arc;

use glyphon::Resolution;
use wgpu::CurrentSurfaceTexture;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::canvas::Canvas;
use crate::canvas::layout::Layout;
use crate::canvas::line::Line;
use crate::canvas::text::TextLayer;
use crate::error::DesktopError;

const BACKGROUND: wgpu::Color = wgpu::Color::BLACK;

pub(crate) struct GpuCanvas {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: wgpu::SurfaceConfiguration,
    lines: Vec<Line>,
    text: Option<TextLayer>,
}

enum AcquiredFrame {
    Ready(wgpu::SurfaceTexture),
    Skipped,
    Outdated,
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
            .ok_or(DesktopError::UnsupportedSurface)?;

        let canvas = Self {
            window,
            surface,
            device,
            queue,
            configuration,
            lines: Vec::new(),
            text: None,
        };
        canvas.configure();
        Ok(canvas)
    }

    fn configure(&self) {
        self.surface.configure(&self.device, &self.configuration);
    }

    fn prepare_text(&mut self) -> Result<bool, DesktopError> {
        if self.lines.is_empty() {
            return Ok(false);
        }

        let size = PhysicalSize::new(self.configuration.width, self.configuration.height);
        let layout = Layout::new(size, self.window.scale_factor());
        let resolution = Resolution {
            width: size.width,
            height: size.height,
        };
        let text = self.text.get_or_insert_with(|| {
            TextLayer::new(&self.device, &self.queue, self.configuration.format)
        });
        text.prepare(&self.device, &self.queue, &self.lines, layout, resolution)?;
        Ok(true)
    }
}

impl Canvas for GpuCanvas {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.configuration.width = size.width;
        self.configuration.height = size.height;
        self.configure();
        self.window.request_redraw();
    }

    fn show(&mut self, lines: Vec<Line>) {
        if self.lines != lines {
            self.lines = lines;
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
        let text = if self.prepare_text()? {
            self.text.as_ref()
        } else {
            None
        };
        let commands = encode_frame(&self.device, &target, text)?;
        self.queue.submit([commands]);
        self.queue.present(frame);

        if let Some(text) = self.text.as_mut() {
            text.trim();
        }
        Ok(())
    }
}

fn device_descriptor() -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        ..Default::default()
    }
}

fn encode_frame(
    device: &wgpu::Device,
    target: &wgpu::TextureView,
    text: Option<&TextLayer>,
) -> Result<wgpu::CommandBuffer, DesktopError> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(BACKGROUND),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    if let Some(text) = text {
        text.render(&mut pass)?;
    }
    drop(pass);
    Ok(encoder.finish())
}

fn acquire(current: CurrentSurfaceTexture) -> Result<AcquiredFrame, DesktopError> {
    match current {
        CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
            Ok(AcquiredFrame::Ready(frame))
        }
        CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
            Ok(AcquiredFrame::Skipped)
        }
        CurrentSurfaceTexture::Outdated => Ok(AcquiredFrame::Outdated),
        CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Validation => {
            Err(DesktopError::SurfaceLost)
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/gpu.rs"]
mod tests;
