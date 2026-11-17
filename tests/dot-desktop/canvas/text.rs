use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::line::Span;

const FRAME: PhysicalSize<u32> = PhysicalSize::new(256, 64);
const RESOLUTION: Resolution = Resolution {
    width: 256,
    height: 64,
};

fn gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("drawing tests need a Metal GPU");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap()
}

fn line(text: &str) -> Vec<Line> {
    vec![vec![Span::new(text, Ink::Full)]]
}

#[test]
fn shapes_again_only_what_changed() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let normal = Layout::new(FRAME, 1.0);
    let retina = Layout::new(FRAME, 2.0);

    text.prepare(&device, &queue, &line("Dot"), normal, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped_lines, line("Dot"));

    text.prepare(&device, &queue, &line("Claude"), normal, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped_lines, line("Claude"));

    text.prepare(&device, &queue, &line("Claude"), retina, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped_layout, Some(retina));
    assert_eq!(text.shaped.len(), 1);
}
