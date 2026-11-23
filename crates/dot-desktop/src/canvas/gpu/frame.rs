use wgpu::CurrentSurfaceTexture;

use crate::canvas::gpu::shape::ShapeLayer;
use crate::canvas::gpu::text::TextLayer;
use crate::error::DesktopError;

const BACKGROUND: wgpu::Color = wgpu::Color::BLACK;

pub(crate) enum AcquiredFrame {
    Ready(wgpu::SurfaceTexture),
    Skipped,
    Outdated,
}

pub(crate) fn encode_frame(
    device: &wgpu::Device,
    target: &wgpu::TextureView,
    shapes: Option<&ShapeLayer>,
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
    if let Some(shapes) = shapes {
        shapes.render_under(&mut pass);
    }
    if let Some(text) = text {
        text.render(&mut pass)?;
    }
    if let Some(shapes) = shapes {
        shapes.render_over(&mut pass);
    }
    drop(pass);
    Ok(encoder.finish())
}

pub(crate) fn acquire(current: CurrentSurfaceTexture) -> Result<AcquiredFrame, DesktopError> {
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
#[path = "../../../../../tests/dot-desktop/canvas/gpu/frame.rs"]
mod tests;
