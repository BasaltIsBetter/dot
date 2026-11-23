use super::*;
use dot_transcript::Ink;

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
    push_rectangle(&mut bytes, &rectangle, FRAME, false);
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
        color: Rgba::ink(Ink::Full),
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
        color: Rgba::ink(Ink::Full),
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
        color: Rgba::ink(Ink::Faint),
    };

    push_rectangle(&mut bytes, &rectangle, FRAME, false);

    let values = floats(&bytes);
    assert_eq!(values.len(), 6 * FLOATS_PER_VERTEX);
    for vertex in values.as_chunks::<FLOATS_PER_VERTEX>().0 {
        assert!(close(vertex[2], 56.0 / 255.0));
        assert!(close(vertex[5], 1.0));
    }
}

#[test]
fn gives_inks_as_they_are_to_a_plain_target() {
    assert_eq!(color(Rgba::ink(Ink::Full), false), [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(color(Rgba::ink(Ink::Live), false)[1], 90.0 / 255.0);
}

#[test]
fn converts_inks_to_linear_light_for_an_srgb_target() {
    let [faint, ..] = color(Rgba::ink(Ink::Faint), true);
    let [ground, ..] = color(Rgba::ink(Ink::Ground), true);
    let [white, ..] = color(Rgba::ink(Ink::Full), true);

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
            color: Rgba::ink(Ink::Faint),
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

    shapes.prepare(&device, &queue, rectangles(16), [], FRAME);
    let first = shapes.vertices.size();
    shapes.prepare(&device, &queue, rectangles(40), [], FRAME);

    assert_eq!(first, 16 * 6 * 24);
    assert_eq!(shapes.vertices.size(), 40 * 6 * 24);
    assert_eq!(shapes.vertex_count, 40 * 6);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}

#[test]
fn keeps_its_buffer_when_the_rectangles_fill_it_exactly() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);
    let first = shapes.vertices.clone();

    shapes.prepare(&device, &queue, rectangles(16), [], FRAME);

    assert_eq!(shapes.vertices, first);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}

#[test]
fn draws_nothing_in_an_empty_frame() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(&device, &queue, rectangles(3), rectangles(2), [0.0, 100.0]);

    assert_eq!(shapes.vertex_count, 0);
}

#[test]
fn passes_alpha_as_it_is_to_a_plain_target() {
    let [.., alpha] = color(Rgba::veil(128), false);

    assert!(close(alpha, 128.0 / 255.0));
}

#[test]
fn raises_alpha_on_an_srgb_target_so_that_a_veil_darkens_like_css() {
    let [.., half] = color(Rgba::veil(128), true);
    let [.., opaque] = color(Rgba::veil(255), true);
    let [.., none] = color(Rgba::veil(0), true);

    let css_half = 128.0_f32 / 255.0;
    assert!(close(half, 1.0 - (1.0 - css_half).powf(2.2)));
    assert!(close(opaque, 1.0));
    assert!(close(none, 0.0));
}

#[test]
fn keeps_the_rectangles_drawn_over_the_text_after_those_drawn_under_it() {
    let (device, queue) = headless_gpu();
    let mut shapes = ShapeLayer::new(&device, FORMAT);

    shapes.prepare(&device, &queue, rectangles(3), rectangles(2), FRAME);

    assert_eq!(shapes.under_count, 3 * 6);
    assert_eq!(shapes.vertex_count, 5 * 6);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}
