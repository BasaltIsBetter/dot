use crate::canvas::mark::{Paint, Rectangle};
use dot_transcript::Palette;

const SHADER: &str = include_str!("shape.wgsl");
const VERTEX_ENTRY: &str = "vertex_main";
const FRAGMENT_ENTRY: &str = "fragment_main";
const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
const CORNERS: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [0.0, 1.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [1.0, 1.0],
];
const FLOATS_PER_VERTEX: usize = 6;
const VERTEX_SIZE: usize = FLOATS_PER_VERTEX * size_of::<f32>();
const VERTEX_STRIDE: wgpu::BufferAddress = VERTEX_SIZE as wgpu::BufferAddress;
const FIRST_RECTANGLES: wgpu::BufferAddress = 16;
const FIRST_CAPACITY: wgpu::BufferAddress =
    FIRST_RECTANGLES * CORNERS.len() as wgpu::BufferAddress * VERTEX_STRIDE;
const SHRINK_WHEN_USING_LESS_THAN_ONE_IN: wgpu::BufferAddress = 4;
const CHANNEL_MAX: f32 = 255.0;
const SRGB_KNEE: f32 = 0.040_45;
const SRGB_SLOPE: f32 = 12.92;
const SRGB_OFFSET: f32 = 0.055;
const SRGB_GAMMA: f32 = 2.4;
const BLEND_GAMMA: f32 = 2.2;

pub(crate) struct ShapeLayer {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    under_count: u32,
    vertex_count: u32,
    bytes: Vec<u8>,
    linear_colors: bool,
}

impl ShapeLayer {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            pipeline: pipeline(device, format),
            vertices: vertex_buffer(device, FIRST_CAPACITY),
            under_count: 0,
            vertex_count: 0,
            bytes: Vec::new(),
            linear_colors: format.is_srgb(),
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        under: impl IntoIterator<Item = Rectangle>,
        over: impl IntoIterator<Item = Rectangle>,
        (frame, palette): ([f32; 2], Palette),
    ) {
        self.bytes.clear();
        let drawable = frame.iter().all(|&side| side > 0.0);
        if drawable {
            self.push_all(under, frame, palette);
        }
        self.under_count = self.count();
        if drawable {
            self.push_all(over, frame, palette);
        }

        let needed = wgpu::BufferAddress::try_from(self.bytes.len()).unwrap_or(u64::MAX);
        if let Some(size) = resized(self.vertices.size(), needed) {
            self.vertices = vertex_buffer(device, size);
        }
        if !self.bytes.is_empty() {
            queue.write_buffer(&self.vertices, 0, &self.bytes);
        }
        let kept = wgpu::BufferAddress::try_from(self.bytes.capacity()).unwrap_or(u64::MAX);
        if far_too_large(kept, needed) {
            self.bytes.shrink_to(self.bytes.len());
        }
        self.vertex_count = self.count();
    }

    pub(crate) fn render_under(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.render(pass, 0..self.under_count);
    }

    pub(crate) fn render_over(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.render(pass, self.under_count..self.vertex_count);
    }

    fn render(&self, pass: &mut wgpu::RenderPass<'_>, vertices: std::ops::Range<u32>) {
        if vertices.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(vertices, 0..1);
    }

    fn push_all(
        &mut self,
        rectangles: impl IntoIterator<Item = Rectangle>,
        frame: [f32; 2],
        palette: Palette,
    ) {
        for rectangle in rectangles {
            let color = color(rectangle.color, palette, self.linear_colors);
            push_rectangle(&mut self.bytes, &rectangle, frame, color);
        }
    }

    fn count(&self) -> u32 {
        u32::try_from(self.bytes.len() / VERTEX_SIZE).unwrap_or(u32::MAX)
    }
}

fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor::default());
    let vertices = wgpu::VertexBufferLayout {
        array_stride: VERTEX_STRIDE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some(VERTEX_ENTRY),
            buffers: &[Some(vertices)],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some(FRAGMENT_ENTRY),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        cache: None,
        multiview_mask: None,
    })
}

fn resized(size: wgpu::BufferAddress, needed: wgpu::BufferAddress) -> Option<wgpu::BufferAddress> {
    let fitting = needed.max(FIRST_CAPACITY).checked_next_power_of_two()?;
    let too_small = needed > size;
    (too_small || far_too_large(size, needed)).then_some(fitting)
}

fn far_too_large(size: wgpu::BufferAddress, needed: wgpu::BufferAddress) -> bool {
    size > FIRST_CAPACITY && needed < size / SHRINK_WHEN_USING_LESS_THAN_ONE_IN
}

fn vertex_buffer(device: &wgpu::Device, size: wgpu::BufferAddress) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn push_rectangle(bytes: &mut Vec<u8>, rectangle: &Rectangle, frame: [f32; 2], color: [f32; 4]) {
    let [frame_width, frame_height] = frame;

    for [across, down] in CORNERS {
        let position = [
            (rectangle.left + across * rectangle.width) / frame_width * 2.0 - 1.0,
            1.0 - (rectangle.top + down * rectangle.height) / frame_height * 2.0,
        ];
        for value in position.iter().chain(&color) {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
}

fn color(color: Paint, palette: Palette, linear: bool) -> [f32; 4] {
    let [red, green, blue] = color.rgb_in(palette).map(|channel| {
        let encoded = f32::from(channel) / CHANNEL_MAX;
        if linear {
            srgb_to_linear(encoded)
        } else {
            encoded
        }
    });
    let alpha = f32::from(color.alpha) / CHANNEL_MAX;
    let alpha = if linear {
        1.0 - (1.0 - alpha).powf(BLEND_GAMMA)
    } else {
        alpha
    };
    [red, green, blue, alpha]
}

fn srgb_to_linear(encoded: f32) -> f32 {
    if encoded <= SRGB_KNEE {
        return encoded / SRGB_SLOPE;
    }
    ((encoded + SRGB_OFFSET) / (1.0 + SRGB_OFFSET)).powf(SRGB_GAMMA)
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/gpu/shape.rs"]
mod tests;
