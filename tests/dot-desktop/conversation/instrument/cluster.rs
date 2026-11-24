use std::time::Duration;

use winit::dpi::PhysicalSize;

use super::*;
use crate::app::tests::IdTranslator;
use dot_transcript::Ink;

const RAILED: PhysicalSize<u32> = PhysicalSize::new(2000, 1400);
const RETINA: f64 = 2.0;

fn layout() -> Layout {
    Layout::reflowed(RAILED, RETINA)
}

fn spark() -> Spark {
    let mut cells = [0; SPARK_COLUMNS];
    cells[SPARK_COLUMNS - 3..].copy_from_slice(&[1, 3, 2]);
    Spark {
        cells,
        rates: [0; SPARK_COLUMNS],
        flash: 0,
    }
}

fn measures() -> Measures {
    Measures {
        tokens: 97,
        rate: Some(65),
        elapsed: Duration::from_millis(2_300),
        spark: spark(),
    }
}

fn reading(measures: Option<Measures>) -> Reading {
    Reading {
        measures,
        heat: 0,
        alpha: u8::MAX,
        values_alpha: u8::MAX,
    }
}

struct Units;

impl Translator for Units {
    fn use_language(&self, _language: dot_api::Language) {}

    fn translate(&self, text: Text) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(match text {
            Text::UnitTokensPerSecond => "j/s",
            Text::UnitSeconds => "s",
            Text::DecimalSeparator => ",",
            other => other.id(),
        })
    }

    fn count(&self, text: Text, _count: u64) -> std::borrow::Cow<'_, str> {
        self.translate(text)
    }
}

fn cluster_of(reading: &Reading) -> (Cluster, Stage, Layout) {
    let layout = layout();
    let stage = Stage::of(&layout);
    let band = stage.reading_band(0.0);
    (
        Cluster::of(reading, &band, &stage, &layout, &Units),
        stage,
        layout,
    )
}

fn text_of(placed: &Placed) -> String {
    placed
        .line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn right_edge(pane: &Pane, placed: &Placed, layout: &Layout) -> f32 {
    let advance = layout.advance_of(TextSize::Small);
    let cells = u16::try_from(columns(&text_of(placed))).unwrap();
    pane.left + f32::from(placed.line.indent + cells) * advance
}

#[test]
fn writes_the_tokens_the_rate_and_the_time() {
    let texts = texts(Some(&measures()), &IdTranslator);

    assert_eq!(
        texts,
        [
            "97".to_owned(),
            "65 unit-tokens-per-second".to_owned(),
            seconds(Duration::from_millis(2_300), &IdTranslator),
        ]
    );
}

#[test]
fn writes_dashes_for_what_is_not_measured_yet() {
    let unmeasured = Measures {
        rate: None,
        ..measures()
    };

    assert_eq!(
        texts(None, &IdTranslator),
        [
            "–––".to_owned(),
            "––– unit-tokens-per-second".to_owned(),
            "–decimal-separator– unit-seconds".to_owned(),
        ]
    );
    assert_eq!(
        texts(Some(&unmeasured), &IdTranslator)[1],
        "––– unit-tokens-per-second"
    );
}

#[test]
fn says_the_measures_on_one_line_for_a_narrow_window() {
    let line = short(&measures(), &IdTranslator);

    assert_eq!(
        line,
        format!(
            "65 unit-tokens-per-second · 97 unit-tokens · {}",
            seconds(Duration::from_millis(2_300), &IdTranslator)
        )
    );
}

#[test]
fn ends_its_values_twenty_four_points_left_of_the_column_on_three_rows_around_the_band() {
    let (cluster, stage, layout) = cluster_of(&reading(Some(measures())));
    let values = &cluster.panes[1];
    let value_right = stage.column[0] - stage.points(24.0);

    for placed in &values.lines {
        assert!((right_edge(values, placed, &layout) - value_right).abs() < 0.01);
    }
    let band = stage.reading_band(0.0);
    let centre = band.top + band.height / 2.0;
    let middle = &values.lines[1];
    let middle_centre = middle.top + layout.line_height_of(TextSize::Small) / 2.0;
    assert!(
        (middle_centre - centre).abs() <= 1.0,
        "{middle_centre} {centre}"
    );
    let row = stage.points(22.0);
    assert!((values.lines[2].top - values.lines[1].top - row).abs() <= 1.0);
    assert!((values.lines[1].top - values.lines[0].top - row).abs() <= 1.0);
    assert_eq!(values.size, TextSize::Small);
}

#[test]
fn ends_its_labels_twelve_points_before_the_values() {
    let (cluster, stage, layout) = cluster_of(&reading(Some(measures())));
    let labels = &cluster.panes[0];
    let advance = layout.advance_of(TextSize::Small);
    let label_right = stage.column[0] - stage.points(24.0) - 7.0 * advance - stage.points(12.0);

    let texts: Vec<String> = labels.lines.iter().map(text_of).collect();
    assert_eq!(texts, ["instrument-tokens", "instrument-time"]);
    for placed in &labels.lines {
        assert!((right_edge(labels, placed, &layout) - label_right).abs() < 0.01);
        assert_eq!(placed.line.spans[0].ink, Ink::Secondary);
    }
}

#[test]
fn draws_the_spark_as_twenty_columns_of_five_cells_ending_at_the_labels() {
    let (cluster, stage, layout) = cluster_of(&reading(Some(measures())));
    let advance = layout.advance_of(TextSize::Small);
    let label_right = stage.column[0] - stage.points(24.0) - 7.0 * advance - stage.points(12.0);

    assert_eq!(cluster.marks.len(), 100);
    let right = cluster
        .marks
        .iter()
        .map(|cell| cell.left + cell.width)
        .fold(f32::MIN, f32::max);
    assert!((right - label_right).abs() <= 1.0, "{right} {label_right}");
    let colours: Vec<Paint> = cluster.marks[80..].iter().map(|cell| cell.color).collect();
    let [dark, floor, lit] = [Ink::Trace, Ink::Faint, Ink::Receding].map(Paint::ink);
    assert_eq!(&colours[0..5], [floor, dark, dark, dark, dark]);
    assert_eq!(&colours[5..10], [lit, dark, dark, dark, dark]);
    assert_eq!(&colours[10..15], [lit, lit, lit, dark, dark]);
    assert_eq!(&colours[15..20], [lit, lit, dark, dark, dark]);
}

#[test]
fn the_newest_lit_cells_flash_orange() {
    let mut flashing = measures();
    flashing.spark.flash = u8::MAX;

    let (cluster, _, _) = cluster_of(&reading(Some(flashing)));

    let newest = &cluster.marks[95..];
    assert_eq!(newest[0].color.rgb(), Ink::Live.rgb());
    assert_eq!(newest[1].color.rgb(), Ink::Live.rgb());
    assert_eq!(newest[2].color, Paint::ink(Ink::Trace));
}

#[test]
fn heats_its_values_and_fades_with_the_instrument() {
    let hot = Reading {
        heat: u8::MAX,
        values_alpha: 0,
        ..reading(Some(measures()))
    };
    let hidden = Reading {
        alpha: 0,
        ..reading(Some(measures()))
    };

    let (hot, _, _) = cluster_of(&hot);
    let (hidden, _, _) = cluster_of(&hidden);

    assert_eq!(hot.panes[1].lines[0].line.spans[0].heat, u8::MAX);
    assert_eq!(hot.panes[1].lines[0].line.alpha, 0);
    assert_eq!(hot.panes[0].lines[0].line.alpha, u8::MAX);
    assert_eq!(hot.marks[0].color, Paint::shaded(Ink::Faint, 0, 0));
    assert_eq!(hidden.panes[0].lines[0].line.alpha, 0);
}

#[test]
fn draws_no_cell_below_the_status_bar() {
    let layout = layout();
    let stage = Stage::of(&layout);
    let low = Rectangle {
        top: stage.floor - stage.points(10.0),
        ..stage.reading_band(0.0)
    };

    let cluster = Cluster::of(
        &reading(Some(measures())),
        &low,
        &stage,
        &layout,
        &IdTranslator,
    );

    assert!(cluster.marks.len() < 100);
    assert!(
        cluster
            .marks
            .iter()
            .all(|cell| cell.top + cell.height <= stage.floor)
    );
}

#[test]
fn centres_the_spark_on_the_band_with_evenly_spaced_columns_of_evenly_spaced_cells() {
    let (cluster, stage, _) = cluster_of(&reading(Some(measures())));
    let band = stage.reading_band(0.0);

    let top = cluster
        .marks
        .iter()
        .map(|cell| cell.top)
        .fold(f32::MAX, f32::min);
    let bottom = cluster
        .marks
        .iter()
        .map(|cell| cell.top + cell.height)
        .fold(f32::MIN, f32::max);
    let middle = band.top + band.height / 2.0;

    assert!(
        (f32::midpoint(top, bottom) - middle).abs() <= 1.5,
        "{top} {bottom} {middle}"
    );
    assert!((bottom - top - stage.exact_points(14.0)).abs() <= 1.0);
    let step = stage.exact_points(4.0);
    for column in 1..20 {
        let left = cluster.marks[column * 5].left;
        let before = cluster.marks[(column - 1) * 5].left;
        assert!(
            (left - before - step).abs() <= 1.0,
            "{column}: {before} {left}"
        );
    }
    let cells: Vec<f32> = cluster.marks[..5].iter().map(|cell| cell.top).collect();
    let cell_step = stage.exact_points(3.0);
    for pair in cells.windows(2) {
        assert!((pair[0] - pair[1] - cell_step).abs() <= 1.0, "{cells:?}");
    }
}

#[test]
fn fades_its_values_by_the_share_of_both_fades() {
    let half = Reading {
        alpha: 128,
        values_alpha: 255,
        ..reading(Some(measures()))
    };

    let (cluster, _, _) = cluster_of(&half);

    assert_eq!(cluster.panes[1].lines[0].line.alpha, 128);
}
