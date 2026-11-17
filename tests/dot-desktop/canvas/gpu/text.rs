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
    vec![Line::new(0, vec![Span::new(text, Ink::Full)])]
}

#[test]
fn shapes_again_only_what_changed() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let normal = Layout::new(FRAME, 1.0);
    let retina = Layout::new(FRAME, 2.0);

    text.prepare(&device, &queue, &line("Dot"), None, normal, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped_lines, line("Dot"));

    text.prepare(&device, &queue, &line("Claude"), None, normal, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped_lines, line("Claude"));

    text.prepare(&device, &queue, &line("Claude"), None, retina, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped_layout, Some(retina));
    assert_eq!(text.shaped.len(), 1);
}

#[test]
fn shapes_the_strip_after_the_lines_and_again_when_it_changes() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let layout = Layout::new(FRAME, 1.0);
    let model = Line::new(0, vec![Span::new("Opus 5.5", Ink::Full)]);
    let cost = Line::new(0, vec![Span::new("$0.00", Ink::Full)]);

    text.prepare(&device, &queue, &[], Some(&model), layout, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped.len(), 1);
    assert_eq!(text.shaped_strip, Some(model));

    text.prepare(
        &device,
        &queue,
        &line("Dot"),
        Some(&cost),
        layout,
        RESOLUTION,
    )
    .unwrap();
    assert_eq!(text.shaped.len(), 2);
    assert_eq!(text.shaped_strip, Some(cost));

    text.prepare(&device, &queue, &line("Dot"), None, layout, RESOLUTION)
        .unwrap();
    assert_eq!(text.shaped.len(), 1);
    assert_eq!(text.shaped_strip, None);
}
