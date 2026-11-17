use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, fontdb,
};

use crate::canvas::layout::Layout;
use crate::canvas::line::{Ink, Line};
use crate::error::DesktopError;

const FONT: &[u8] = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");
const FAMILY: &str = "JetBrains Mono";
const SHAPING_LOCALE: &str = "en-US";

pub(crate) struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    shaped: Vec<Buffer>,
    shaped_lines: Vec<Line>,
    shaped_layout: Option<Layout>,
}

impl TextLayer {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let mut fonts = fontdb::Database::new();
        fonts.load_font_data(FONT.to_vec());
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
            shaped: Vec::new(),
            shaped_lines: Vec::new(),
            shaped_layout: None,
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lines: &[Line],
        layout: Layout,
        resolution: Resolution,
    ) -> Result<(), DesktopError> {
        if self.shaped_lines != lines || self.shaped_layout != Some(layout) {
            self.shape(lines, layout);
        }
        self.viewport.update(queue, resolution);

        let bounds = TextBounds {
            left: 0,
            top: 0,
            right: i32::try_from(resolution.width).unwrap_or(i32::MAX),
            bottom: i32::try_from(resolution.height).unwrap_or(i32::MAX),
        };
        let areas = self
            .shaped
            .iter()
            .zip(lines)
            .enumerate()
            .map(|(index, (buffer, line))| TextArea {
                buffer,
                left: layout.x(line.indent),
                top: layout.line_top(index),
                scale: 1.0,
                bounds,
                default_color: color(Ink::Full),
                custom_glyphs: &[],
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

    fn shape(&mut self, lines: &[Line], layout: Layout) {
        let metrics = Metrics::new(layout.font_size(), layout.line_height());
        let font = Attrs::new().family(Family::Name(FAMILY));

        self.shaped = lines
            .iter()
            .map(|line| {
                let mut buffer = Buffer::new(&mut self.font_system, metrics);
                let spans = line
                    .spans
                    .iter()
                    .map(|span| (span.text.as_str(), font.clone().color(color(span.ink))));
                buffer.set_rich_text(spans, &font, Shaping::Basic, None);
                buffer.shape_until_scroll(&mut self.font_system, false);
                buffer
            })
            .collect();
        self.shaped_lines = lines.to_vec();
        self.shaped_layout = Some(layout);
    }
}

fn color(ink: Ink) -> Color {
    let [red, green, blue] = ink.rgb();
    Color::rgb(red, green, blue)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/canvas/text.rs"]
mod tests;
