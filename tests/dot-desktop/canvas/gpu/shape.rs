use super::*;
use dot_transcript::{Ink, Palette};

const FRAME: [f32; 2] = [200.0, 100.0];

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_ne_bytes(*chunk))
        .collect()
}

fn corners_of(rectangle: Rectangle) -> Vec<[f32; 2]> {
    let mut bytes = Vec::new();
    push_rectangle(
        &mut bytes,
        &rectangle,
        FRAME,
        color(rectangle.color, Palette::default(), false),
    );
    floats(&bytes)
        .as_chunks::<FLOATS_PER_VERTEX>()
        .0
        .iter()
        .map(|vertex| [vertex[0], vertex[1]])
        .collect()
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.0001
}

#[test]
fn covers_the_whole_frame_with_a_frame_sized_rectangle() {
    let corners = corners_of(Rectangle {
        left: 0.0,
        top: 0.0,
        width: 200.0,
        height: 100.0,
        color: Paint::ink(Ink::Full),
    });

    assert_eq!(
        corners,
        [
            [-1.0, 1.0],
            [1.0, 1.0],
            [-1.0, -1.0],
            [-1.0, -1.0],
            [1.0, 1.0],
            [1.0, -1.0],
        ]
    );
}

#[test]
fn places_a_rectangle_from_the_top_left_corner() {
    let corners = corners_of(Rectangle {
        left: 50.0,
        top: 25.0,
        width: 100.0,
        height: 50.0,
        color: Paint::ink(Ink::Full),
    });

    assert!(close(corners[0][0], -0.5) && close(corners[0][1], 0.5));
    assert!(close(corners[5][0], 0.5) && close(corners[5][1], -0.5));
}

#[test]
fn writes_six_opaque_vertices_of_the_ink_per_rectangle() {
    let mut bytes = Vec::new();
    let rectangle = Rectangle {
        left: 0.0,
        top: 0.0,
        width: 1.0,
        height: 1.0,
        color: Paint::ink(Ink::Faint),
    };

    push_rectangle(
        &mut bytes,
        &rectangle,
        FRAME,
        color(rectangle.color, Palette::default(), false),
    );

    let values = floats(&bytes);
    assert_eq!(values.len(), 6 * FLOATS_PER_VERTEX);
    for vertex in values.as_chunks::<FLOATS_PER_VERTEX>().0 {
        assert!(close(vertex[2], 56.0 / 255.0));
        assert!(close(vertex[5], 1.0));
    }
}

#[test]
fn gives_inks_as_they_are_to_a_plain_target() {
    assert_eq!(
        color(Paint::ink(Ink::Full), Palette::default(), false),
        [1.0, 1.0, 1.0, 1.0]
    );
    assert_eq!(
        color(Paint::ink(Ink::Live), Palette::default(), false)[1],
        90.0 / 255.0
    );
}

#[test]
fn converts_inks_to_linear_light_for_an_srgb_target() {
    let [faint, ..] = color(Paint::ink(Ink::Faint), Palette::default(), true);
    let [ground, ..] = color(Paint::ink(Ink::Ground), Palette::default(), true);
    let [white, ..] = color(Paint::ink(Ink::Full), Palette::default(), true);

    assert!(close(faint, 0.039_546));
    assert!(close(ground, 0.010_330));
    assert!(close(white, 1.0));
    assert!(close(srgb_to_linear(0.04), 0.04 / 12.92));
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

fn headless_gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("drawing tests need a Metal GPU");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap()
}

fn rectangles(count: usize) -> Vec<Rectangle> {
    (0..count)
        .map(|index| Rectangle {
            left: f32::from(u8::try_from(index).unwrap()),
            top: 0.0,
            width: 1.0,
            height: 1.0,
            color: Paint::ink(Ink::Faint),
        })
        .collect()
}

#[test]
fn starts_with_room_for_sixteen_rectangles() {
    let (device, _queue) = headless_gpu();

    let shapes = ShapeLayer::new(&device, FORMAT);

    assert_eq!(shapes.vertices.size(), 16 * 6 * 24);
}

#[test]
fn grows_its_buffer_only_when_the_rectangles_outgrow_it() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(
        &device,
        &queue,
        rectangles(16),
        [],
        (FRAME, Palette::default()),
    );
    let first = shapes.vertices.size();
    shapes.prepare(
        &device,
        &queue,
        rectangles(40),
        [],
        (FRAME, Palette::default()),
    );

    assert_eq!(first, 16 * 6 * 24);
    assert_eq!(
        shapes.vertices.size(),
        (40 * 6 * 24_u64).next_power_of_two()
    );
    assert_eq!(shapes.vertex_count, 40 * 6);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}

#[test]
fn keeps_its_buffer_when_the_rectangles_fill_it_exactly() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);
    let first = shapes.vertices.clone();

    shapes.prepare(
        &device,
        &queue,
        rectangles(16),
        [],
        (FRAME, Palette::default()),
    );

    assert_eq!(shapes.vertices, first);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}

#[test]
fn draws_nothing_in_an_empty_frame() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(
        &device,
        &queue,
        rectangles(3),
        rectangles(2),
        ([0.0, 100.0], Palette::default()),
    );

    assert_eq!(shapes.vertex_count, 0);
}

#[test]
fn passes_alpha_as_it_is_to_a_plain_target() {
    let [.., alpha] = color(Paint::veil(128), Palette::default(), false);

    assert!(close(alpha, 128.0 / 255.0));
}

#[test]
fn raises_alpha_on_an_srgb_target_so_that_a_veil_darkens_like_css() {
    let [.., half] = color(Paint::veil(128), Palette::default(), true);
    let [.., opaque] = color(Paint::veil(255), Palette::default(), true);
    let [.., none] = color(Paint::veil(0), Palette::default(), true);

    let css_half = 128.0_f32 / 255.0;
    assert!(close(half, 1.0 - (1.0 - css_half).powf(2.2)));
    assert!(close(opaque, 1.0));
    assert!(close(none, 0.0));
}

#[test]
fn keeps_the_rectangles_drawn_over_the_text_after_those_drawn_under_it() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(
        &device,
        &queue,
        rectangles(3),
        rectangles(2),
        (FRAME, Palette::default()),
    );

    assert_eq!(shapes.under_count, 3 * 6);
    assert_eq!(shapes.vertex_count, 5 * 6);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}

#[test]
fn grows_its_vertices_by_powers_of_two_and_gives_back_what_it_no_longer_needs() {
    let first = FIRST_CAPACITY;

    assert_eq!(resized(first, first), None);
    assert_eq!(
        resized(first, first + 1),
        Some((first + 1).next_power_of_two())
    );
    assert_eq!(resized(1 << 20, 600_000), None);
    assert_eq!(resized(1 << 20, 200_000), Some(262_144));
    assert_eq!(resized(first, 0), None);
    assert_eq!(resized(1 << 20, 0), Some(first.next_power_of_two()));
}
