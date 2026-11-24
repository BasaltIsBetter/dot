use super::*;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

#[test]
fn keeps_one_frame_in_flight_so_the_window_holds_two_images() {
    let default = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: FORMAT,
        width: 2816,
        height: 1542,
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
        color_space: wgpu::SurfaceColorSpace::Auto,
    };

    let configured = with_one_frame_in_flight(default.clone());

    assert_eq!(configured.desired_maximum_frame_latency, 1);
    assert_eq!(configured.width, default.width);
    assert_eq!(configured.format, default.format);
}

#[test]
fn draws_on_a_surface_the_device_can_hold_and_never_an_empty_one() {
    assert_eq!(
        drawable(PhysicalSize::new(2880, 1800), 16_384),
        (2880, 1800)
    );
    assert_eq!(
        drawable(PhysicalSize::new(17_000, 900), 16_384),
        (16_384, 900)
    );
    assert_eq!(drawable(PhysicalSize::new(0, 0), 16_384), (1, 1));
    assert_eq!(drawable(PhysicalSize::new(10, 10), 0), (1, 1));
}
