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

fn line(text: &str) -> Line {
    Line::new(0, vec![Span::new(text, Ink::Full)])
}

fn runs(lines: &[Line]) -> Vec<Run<'_>> {
    lines
        .iter()
        .map(|line| Run {
            line,
            left: 0.0,
            top: 0.0,
            clip: [0.0, 64.0],
        })
        .collect()
}

fn shape_of(text: &TextLayer, line: &Line) -> *const glyphon::cosmic_text::BufferLine {
    text.shaped[line].lines.as_ptr()
}

#[test]
fn reuses_the_shape_of_a_line_still_shown() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let layout = Layout::zoomed(FRAME, 1.0);
    let first = [line("Dot"), line("Claude")];
    let second = [line("Claude"), line("Opus")];

    text.prepare(&device, &queue, &runs(&first), &layout, RESOLUTION)
        .unwrap();
    let claude = shape_of(&text, &line("Claude"));
    text.prepare(&device, &queue, &runs(&second), &layout, RESOLUTION)
        .unwrap();

    assert_eq!(shape_of(&text, &line("Claude")), claude);
    assert!(text.shaped.contains_key(&line("Opus")));
    assert!(!text.shaped.contains_key(&line("Dot")));
    assert!(text.unused.is_empty());
}

#[test]
fn shapes_everything_again_when_the_text_size_changes() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let lines = [line("Claude")];
    let retina = Layout::zoomed(FRAME, 2.0);
    let retina_metrics = Metrics::new(retina.font_size(), retina.line_height());

    text.prepare(
        &device,
        &queue,
        &runs(&lines),
        &Layout::zoomed(FRAME, 1.0),
        RESOLUTION,
    )
    .unwrap();
    text.prepare(&device, &queue, &runs(&lines), &retina, RESOLUTION)
        .unwrap();

    assert_eq!(text.shaped[&lines[0]].metrics(), retina_metrics);
    assert_eq!(text.metrics, Some(retina_metrics));
}

#[test]
fn shapes_identical_lines_once() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let lines = [line("_"), line("_"), line("Dot")];

    text.prepare(
        &device,
        &queue,
        &runs(&lines),
        &Layout::zoomed(FRAME, 1.0),
        RESOLUTION,
    )
    .unwrap();

    assert_eq!(text.shaped.len(), 2);
}

#[test]
fn a_change_of_heat_is_a_new_shape() {
    let (device, queue) = gpu();
    let mut text = TextLayer::new(&device, &queue, wgpu::TextureFormat::Bgra8Unorm);
    let hot = Line::new(0, vec![Span::new("Dot", Ink::Full).heated(200)]);
    let cooler = Line::new(0, vec![Span::new("Dot", Ink::Full).heated(100)]);

    text.prepare(
        &device,
        &queue,
        &runs(std::slice::from_ref(&hot)),
        &Layout::zoomed(FRAME, 1.0),
        RESOLUTION,
    )
    .unwrap();
    text.prepare(
        &device,
        &queue,
        &runs(std::slice::from_ref(&cooler)),
        &Layout::zoomed(FRAME, 1.0),
        RESOLUTION,
    )
    .unwrap();

    assert!(text.shaped.contains_key(&cooler));
    assert!(!text.shaped.contains_key(&hot));
}

#[test]
fn rounds_clip_edges_to_whole_pixels() {
    assert_eq!(pixel(12.4), 12);
    assert_eq!(pixel(12.6), 13);
    assert_eq!(pixel(-3.0), -3);
}

#[test]
fn draws_arrows_box_drawing_math_greek_and_the_replacement_character() {
    let mut fonts = fontdb::Database::new();
    fonts.load_font_source(fontdb::Source::Binary(Arc::new(FONT)));
    let mut font_system = FontSystem::new_with_locale_and_db(SHAPING_LOCALE.to_owned(), fonts);
    let drawn = "→←↑↓✓✗≥≤≠±×÷├──└│┌┐┘█░λΣπ€…•�⌘";

    let buffer = shape(&mut font_system, &line(drawn), Metrics::new(13.0, 20.0));

    let glyphs: Vec<u16> = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.glyph_id))
        .collect();
    assert_eq!(glyphs.len(), drawn.chars().count());
    assert!(
        !glyphs.contains(&0),
        "a character fell back to the empty glyph"
    );
}

#[test]
fn a_character_the_font_lacks_falls_back_to_the_empty_glyph() {
    let mut fonts = fontdb::Database::new();
    fonts.load_font_source(fontdb::Source::Binary(Arc::new(FONT)));
    let mut font_system = FontSystem::new_with_locale_and_db(SHAPING_LOCALE.to_owned(), fonts);

    let buffer = shape(&mut font_system, &line("⠿"), Metrics::new(13.0, 20.0));

    let glyphs: Vec<u16> = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.glyph_id))
        .collect();
    assert_eq!(glyphs, [0]);
}
