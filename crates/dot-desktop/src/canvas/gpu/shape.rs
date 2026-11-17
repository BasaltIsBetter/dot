use crate::canvas::line::Ink;
use crate::canvas::strip::Rectangle;

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
const CHANNEL_MAX: f32 = 255.0;
const OPAQUE: f32 = 1.0;
const SRGB_KNEE: f32 = 0.040_45;
const SRGB_SLOPE: f32 = 12.92;
const SRGB_OFFSET: f32 = 0.055;
const SRGB_GAMMA: f32 = 2.4;

pub(crate) struct ShapeLayer {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    vertex_count: u32,
    bytes: Vec<u8>,
    linear_colors: bool,
}

impl ShapeLayer {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            pipeline: pipeline(device, format),
            vertices: vertex_buffer(device, FIRST_CAPACITY),
            vertex_count: 0,
            bytes: Vec::new(),
            linear_colors: format.is_srgb(),
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rectangles: &[Rectangle],
        frame: [f32; 2],
    ) {
        self.bytes.clear();
        if frame.iter().all(|&side| side > 0.0) {
            for rectangle in rectangles {
                push_rectangle(&mut self.bytes, rectangle, frame, self.linear_colors);
            }
        }

        let needed = wgpu::BufferAddress::try_from(self.bytes.len()).unwrap_or(u64::MAX);
        if needed > self.vertices.size() {
            self.vertices = vertex_buffer(device, needed);
        }
        if !self.bytes.is_empty() {
            queue.write_buffer(&self.vertices, 0, &self.bytes);
        }
        self.vertex_count = u32::try_from(self.bytes.len() / VERTEX_SIZE).unwrap_or(u32::MAX);
    }

    pub(crate) fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
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
                blend: None,
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

fn vertex_buffer(device: &wgpu::Device, size: wgpu::BufferAddress) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn push_rectangle(bytes: &mut Vec<u8>, rectangle: &Rectangle, frame: [f32; 2], linear: bool) {
    let [frame_width, frame_height] = frame;
    let [red, green, blue, alpha] = color(rectangle.ink, linear);

    for [across, down] in CORNERS {
        let x = (rectangle.left + across * rectangle.width) / frame_width * 2.0 - 1.0;
        let y = 1.0 - (rectangle.top + down * rectangle.height) / frame_height * 2.0;
        for value in [x, y, red, green, blue, alpha] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
}

fn color(ink: Ink, linear: bool) -> [f32; 4] {
    let [red, green, blue] = ink.rgb().map(|channel| {
        let encoded = f32::from(channel) / CHANNEL_MAX;
        if linear {
            srgb_to_linear(encoded)
        } else {
            encoded
        }
    });
    [red, green, blue, OPAQUE]
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
