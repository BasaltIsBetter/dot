use glyphon::Resolution;
use winit::dpi::PhysicalSize;

use super::*;
use crate::canvas::layout::Layout;
use crate::canvas::line::{Ink, Line, Span};
use crate::canvas::mark::{Gauge, Reticle, Strip};
use crate::canvas::scene::Run;
use winit::dpi::PhysicalPosition;

const BYTES_PER_PIXEL: u32 = 4;
const NARROW_FRAME: PhysicalSize<u32> =
    PhysicalSize::new(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT / BYTES_PER_PIXEL, 4);
const TEXT_FRAME: PhysicalSize<u32> = PhysicalSize::new(256, 32);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const OPAQUE_BLACK: [u8; 4] = [0, 0, 0, 255];
const STANDARD_SCREEN: f64 = 1.0;

fn headless_gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("drawing tests need a Metal GPU");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap()
}

fn offscreen_frame(device: &wgpu::Device, size: PhysicalSize<u32>) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn read_pixels(device: &wgpu::Device, queue: &wgpu::Queue, frame: &wgpu::Texture) -> Vec<[u8; 4]> {
    let bytes_per_row = frame.width() * BYTES_PER_PIXEL;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(bytes_per_row * frame.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        frame.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
        },
        frame.size(),
    );
    queue.submit([encoder.finish()]);

    readback.map_async(wgpu::MapMode::Read, .., |mapped| mapped.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    readback
        .get_mapped_range(..)
        .unwrap()
        .as_chunks()
        .0
        .to_vec()
}

fn draw_text(ink: Ink) -> Vec<[u8; 4]> {
    draw_span(Span::new("Dot", ink), [0.0, 32.0])
}

fn draw_span(span: Span, clip: [f32; 2]) -> Vec<[u8; 4]> {
    let (device, queue) = headless_gpu();
    let frame = offscreen_frame(&device, TEXT_FRAME);
    let target = frame.create_view(&wgpu::TextureViewDescriptor::default());
    let mut text = TextLayer::new(&device, &queue, FORMAT);
    let line = Line::new(0, vec![span]);
    let runs = [Run {
        line: &line,
        left: 0.0,
        top: 0.0,
        clip,
    }];
    let resolution = Resolution {
        width: TEXT_FRAME.width,
        height: TEXT_FRAME.height,
    };

    text.prepare(
        &device,
        &queue,
        &runs,
        &Layout::reflowed(TEXT_FRAME, STANDARD_SCREEN),
        resolution,
    )
    .unwrap();
    queue.submit([encode_frame(&device, &target, None, Some(&text)).unwrap()]);

    read_pixels(&device, &queue, &frame)
}

#[test]
fn paints_the_whole_frame_opaque_black() {
    let (device, queue) = headless_gpu();
    let frame = offscreen_frame(&device, NARROW_FRAME);
    let target = frame.create_view(&wgpu::TextureViewDescriptor::default());

    queue.submit([encode_frame(&device, &target, None, None).unwrap()]);

    let pixels = read_pixels(&device, &queue, &frame);
    assert!(pixels.iter().all(|pixel| *pixel == OPAQUE_BLACK));
}

#[test]
fn an_unpainted_frame_is_not_opaque_black() {
    let (device, queue) = headless_gpu();
    let frame = offscreen_frame(&device, NARROW_FRAME);

    let pixels = read_pixels(&device, &queue, &frame);

    assert!(pixels.iter().all(|pixel| *pixel != OPAQUE_BLACK));
}

#[test]
fn draws_full_ink_text_in_white_on_black() {
    let pixels = draw_text(Ink::Full);

    let [blue, green, red, _] = *pixels.iter().max_by_key(|pixel| pixel[2]).unwrap();
    assert!(red > 200, "the brightest pixel is {red}");
    assert_eq!((blue, green), (red, red));
    assert!(
        pixels
            .iter()
            .filter(|pixel| **pixel == OPAQUE_BLACK)
            .count()
            > pixels.len() / 2
    );
}

#[test]
fn draws_live_text_in_orange() {
    let pixels = draw_text(Ink::Live);

    let [blue, green, red, _] = *pixels.iter().max_by_key(|pixel| pixel[2]).unwrap();
    assert!(red > 200, "the reddest pixel is {red}");
    assert!(green < red && blue < green);
}

#[test]
fn draws_cooling_text_between_orange_and_white() {
    let pixels = draw_span(Span::new("Dot", Ink::Full).heated(128), [0.0, 32.0]);

    let [blue, green, red, _] = *pixels.iter().max_by_key(|pixel| pixel[2]).unwrap();
    assert!(red > 200, "the reddest pixel is {red}");
    assert!(green > 60 && green < 200, "green {green}");
    assert!(blue > 40 && blue < green, "blue {blue}");
}

#[test]
fn draws_nothing_outside_the_clip() {
    let pixels = draw_span(Span::new("Dot", Ink::Full), [24.0, 32.0]);

    assert!(pixels.iter().all(|pixel| *pixel == OPAQUE_BLACK));
}

#[test]
fn skips_the_frame_when_the_gpu_is_late() {
    let acquired = acquire(CurrentSurfaceTexture::Timeout);

    assert!(matches!(acquired, Ok(AcquiredFrame::Skipped)));
}

#[test]
fn skips_the_frame_while_the_window_is_hidden() {
    let acquired = acquire(CurrentSurfaceTexture::Occluded);

    assert!(matches!(acquired, Ok(AcquiredFrame::Skipped)));
}

#[test]
fn asks_for_a_new_configuration_when_the_surface_is_outdated() {
    let acquired = acquire(CurrentSurfaceTexture::Outdated);

    assert!(matches!(acquired, Ok(AcquiredFrame::Outdated)));
}

#[test]
fn fails_when_the_surface_is_lost() {
    let acquired = acquire(CurrentSurfaceTexture::Lost);

    assert!(matches!(acquired, Err(DesktopError::SurfaceLost)));
}

#[test]
fn fails_when_the_surface_is_rejected_by_validation() {
    let acquired = acquire(CurrentSurfaceTexture::Validation);

    assert!(matches!(acquired, Err(DesktopError::SurfaceLost)));
}

const STRIP_FRAME: PhysicalSize<u32> = PhysicalSize::new(256, 64);

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the coordinates are small pixel positions inside the test frame"
)]
fn pixel_at(pixels: &[[u8; 4]], x: f32, y: f32) -> [u8; 4] {
    let [x, y] = [x, y].map(|coordinate| usize::try_from(coordinate as u32).unwrap());
    let width = usize::try_from(STRIP_FRAME.width).unwrap();
    pixels[y * width + x]
}

fn opaque(ink: Ink) -> [u8; 4] {
    let [red, green, blue] = ink.rgb();
    [blue, green, red, 255]
}

#[test]
fn draws_the_strip_rules_and_gauge_in_their_inks() {
    let (device, queue) = headless_gpu();
    let frame = offscreen_frame(&device, STRIP_FRAME);
    let target = frame.create_view(&wgpu::TextureViewDescriptor::default());
    let layout = Layout::zoomed(STRIP_FRAME, 1.0);
    let strip = Strip {
        rules: vec![10],
        gauge: Some(Gauge {
            column: 20,
            width: 6,
            fill_per_mille: 250,
            ink: Ink::Receding,
        }),
        ..Strip::default()
    };
    let rectangles = strip.rectangles(&layout);
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(
        &device,
        &queue,
        rectangles.iter().copied(),
        [],
        [layout.width(), layout.height()],
    );
    queue.submit([encode_frame(&device, &target, Some(&shapes), None).unwrap()]);

    let pixels = read_pixels(&device, &queue, &frame);
    for rectangle in &rectangles {
        let center = pixel_at(
            &pixels,
            rectangle.left + rectangle.width / 2.0,
            rectangle.top + rectangle.height / 2.0,
        );
        assert_eq!(center, opaque(rectangle.ink), "{rectangle:?}");
    }
    let track = rectangles[rectangles.len() - 2];
    let unfilled = pixel_at(&pixels, track.left + track.width - 1.0, track.top);
    assert_eq!(unfilled, opaque(Ink::Ground));
    assert_eq!(
        pixel_at(&pixels, 1.0, layout.strip_top() - 1.0),
        OPAQUE_BLACK
    );
}

#[test]
fn draws_a_hollow_ring_and_its_crossing_hairlines() {
    let (device, queue) = headless_gpu();
    let frame = offscreen_frame(&device, STRIP_FRAME);
    let target = frame.create_view(&wgpu::TextureViewDescriptor::default());
    let reticle = Reticle::at(PhysicalPosition::new(100.0, 30.0), 1.0);
    let [width, height] = [256.0, 64.0];
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(
        &device,
        &queue,
        reticle.hairlines(width, height),
        [reticle.ring()],
        [width, height],
    );
    queue.submit([encode_frame(&device, &target, Some(&shapes), None).unwrap()]);

    let pixels = read_pixels(&device, &queue, &frame);
    assert_eq!(pixel_at(&pixels, 100.0, 30.0), OPAQUE_BLACK);
    assert_eq!(pixel_at(&pixels, 104.0, 30.0), OPAQUE_BLACK);
    assert_eq!(pixel_at(&pixels, 100.0, 22.0), opaque(Ink::Full));
    assert_eq!(pixel_at(&pixels, 108.0, 30.0), opaque(Ink::Full));
    assert_eq!(pixel_at(&pixels, 100.0, 5.0), opaque(Ink::Ground));
    assert_eq!(pixel_at(&pixels, 200.0, 30.0), opaque(Ink::Ground));
    assert_eq!(pixel_at(&pixels, 150.0, 5.0), OPAQUE_BLACK);
}
