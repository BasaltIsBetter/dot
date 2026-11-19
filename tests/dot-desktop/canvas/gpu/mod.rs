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
