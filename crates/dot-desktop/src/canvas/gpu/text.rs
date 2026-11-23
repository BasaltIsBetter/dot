use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, fontdb,
};

use crate::canvas::layout::{Layout, TextSize};
use crate::canvas::scene::Run;
use crate::error::DesktopError;
use dot_transcript::{Ink, Line};

const FONT: &[u8] = include_bytes!("../../../assets/JetBrainsMono-Regular.ttf");
const FAMILY: &str = "JetBrains Mono";
const SHAPING_LOCALE: &str = "en-US";
const SIZES: [TextSize; 3] = [TextSize::Regular, TextSize::Small, TextSize::Tiny];

pub(crate) struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    shelves: [Shelf; SIZES.len()],
}

#[derive(Default)]
struct Shelf {
    shaped: HashMap<Line, Buffer>,
    unused: HashMap<Line, Buffer>,
    metrics: Option<Metrics>,
}

impl TextLayer {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let mut fonts = fontdb::Database::new();
        fonts.load_font_source(fontdb::Source::Binary(Arc::new(FONT)));
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            font_system: FontSystem::new_with_locale_and_db(SHAPING_LOCALE.to_owned(), fonts),
            swash_cache: SwashCache::new(),
            viewport: Viewport::new(device, &cache),
            atlas,
            renderer,
            shelves: Default::default(),
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        runs: &[Run<'_>],
        layout: &Layout,
        resolution: Resolution,
    ) -> Result<(), DesktopError> {
        for size in SIZES {
            let metrics = Metrics::new(layout.font_size_of(size), layout.line_height_of(size));
            self.shelves[shelf_of(size)].keep_shaped(&mut self.font_system, runs, size, metrics);
        }
        self.viewport.update(queue, resolution);

        let right = i32::try_from(resolution.width).unwrap_or(i32::MAX);
        let shelves = &self.shelves;
        let areas = runs.iter().filter_map(|run| {
            Some(TextArea {
                buffer: shelves[shelf_of(run.size)].shaped.get(run.line)?,
                left: run.left,
                top: run.top,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: pixel(run.clip[0]),
                    right,
                    bottom: pixel(run.clip[1]),
                },
                default_color: color(Ink::Full.rgb()),
                custom_glyphs: &[],
            })
        });

        self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        )?;
        Ok(())
    }

    pub(crate) fn render(&self, pass: &mut wgpu::RenderPass<'_>) -> Result<(), DesktopError> {
        self.renderer.render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }

    pub(crate) fn trim(&mut self) {
        self.atlas.trim();
    }
}

impl Shelf {
    fn keep_shaped(
        &mut self,
        font_system: &mut FontSystem,
        runs: &[Run<'_>],
        size: TextSize,
        metrics: Metrics,
    ) {
        mem::swap(&mut self.shaped, &mut self.unused);
        if self.metrics != Some(metrics) {
            self.unused.clear();
            self.metrics = Some(metrics);
        }

        for run in runs.iter().filter(|run| run.size == size) {
            if self.shaped.contains_key(run.line) {
                continue;
            }
            let (line, buffer) = self.unused.remove_entry(run.line).unwrap_or_else(|| {
                let buffer = shape(font_system, run.line, metrics);
                (run.line.clone(), buffer)
            });
            self.shaped.insert(line, buffer);
        }
        self.unused.clear();
    }
}

const fn shelf_of(size: TextSize) -> usize {
    match size {
        TextSize::Regular => 0,
        TextSize::Small => 1,
        TextSize::Tiny => 2,
    }
}

fn shape(font_system: &mut FontSystem, line: &Line, metrics: Metrics) -> Buffer {
    let font = Attrs::new().family(Family::Name(FAMILY));
    let mut buffer = Buffer::new(font_system, metrics);
    let spans = line.spans.iter().map(|span| {
        (
            span.text.as_str(),
            font.clone().color(color(line.rgb_of(span))),
        )
    });
    buffer.set_rich_text(spans, &font, Shaping::Basic, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

fn color([red, green, blue]: [u8; 3]) -> Color {
    Color::rgb(red, green, blue)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "clip edges are window coordinates, far inside the range of i32; the cast saturates"
)]
fn pixel(coordinate: f32) -> i32 {
    coordinate.round() as i32
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/canvas/gpu/text.rs"]
mod tests;
