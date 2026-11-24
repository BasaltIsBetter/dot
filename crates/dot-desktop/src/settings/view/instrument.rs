use std::time::{Duration, Instant};

use dot_api::{Text, Translator};
use dot_transcript::{Ink, Line, Motion, Span, opacity, rows, to_f32};

use crate::canvas::{Layout, Paint, Pane, Rectangle, TextSize};
use crate::settings::catalog::{Kind, Setting};
use crate::settings::reel::Reel;
use crate::settings::view::Look;
use crate::settings::view::draw::{Frame, ROW, pane, rectangle, width_of};

const BAND_INK: Ink = Ink::Trace;
const HINT_INK: Ink = Ink::Secondary;
const NAME_INK: Ink = Ink::Secondary;
const VALUE_INK: Ink = Ink::Full;
const ON_INK: Ink = Ink::Full;
const DEFAULT_INK: Ink = Ink::Faint;
const OFF_INK: Ink = Ink::Trace;
const HINT_BELOW_BAND: f32 = 1.0;
const FROM_THE_COLUMN: f32 = 32.0;
const INSTRUMENT_ROW: f32 = 22.0;
const INSTRUMENT_ABOVE_MIDDLE: f32 = 33.0;
const NAME_GAP: &str = "  ";
const BLOCK_WIDTH: f32 = 7.0;
const BLOCK_HEIGHT: f32 = 12.0;
const BLOCK_GAP: f32 = 2.0;
const SWATCH_RESTING: f64 = 0.3;
const FLASH: Motion = Motion::landing(Duration::from_millis(450));

pub(crate) fn band(layout: &Layout, frame: &Frame) -> Rectangle {
    rectangle(
        [0.0, frame.band_top, layout.width(), layout.scaled(ROW)],
        Paint::ink(BAND_INK),
    )
}

pub(crate) fn hint(
    setting: Setting,
    translator: &impl Translator,
    (layout, frame): (&Layout, &Frame),
) -> Pane {
    let text = translator.translate(setting.hint());
    let top = frame.band_top + layout.scaled(ROW + HINT_BELOW_BAND);
    pane(
        frame.left,
        TextSize::Small,
        top,
        Line::new(0, vec![Span::new(text.as_ref(), HINT_INK)]),
    )
}

pub(crate) fn instrument<T: Translator>(
    reel: &Reel,
    look: &Look<'_, T>,
    (layout, frame): (&Layout, &Frame),
    now: Instant,
) -> (Vec<Pane>, Vec<Rectangle>) {
    let setting = reel.focus();
    let right = frame.left - layout.scaled(FROM_THE_COLUMN);
    let row = layout.scaled(INSTRUMENT_ROW);
    let top = frame.band_top + layout.scaled(ROW) / 2.0 - layout.scaled(INSTRUMENT_ABOVE_MIDDLE);
    let translator = look.translator;

    let default = setting.option_name(setting.chosen(look.defaults), translator);
    let (rank, total) = reel.rank();
    let acts = setting.kind() == Kind::Action;
    let panes = [
        (0.0, Text::SettingsDefault, default),
        (2.0, Text::SettingsRank, format!("{rank} / {total}")),
    ]
    .into_iter()
    .filter(|(_, name, _)| !acts || *name == Text::SettingsRank)
    .map(|(place, name, value)| {
        let name = translator.translate(name);
        let spans = vec![
            Span::new(format!("{name}{NAME_GAP}"), NAME_INK),
            Span::new(value, VALUE_INK),
        ];
        let width = spans.iter().map(|span| width_of(&span.text)).sum::<f32>()
            * layout.advance_of(TextSize::Small);
        let text_top = layout.text_top(TextSize::Small, top + place * row, row);
        pane(
            right - width,
            TextSize::Small,
            text_top,
            Line::new(0, spans),
        )
    })
    .collect();

    let flash = look
        .changed
        .filter(|(changed, _)| *changed == setting)
        .and_then(|(_, at)| FLASH.eased(now.saturating_duration_since(at)))
        .map_or(0, |eased| opacity(1.0 - eased));
    let meter_top = (top + row + (row - layout.scaled(BLOCK_HEIGHT)) / 2.0).round();
    if acts {
        return (panes, Vec::new());
    }
    (
        panes,
        meter(setting, look, (layout, right, meter_top), flash),
    )
}

fn meter<T: Translator>(
    setting: Setting,
    look: &Look<'_, T>,
    (layout, right, top): (&Layout, f32, f32),
    flash: u8,
) -> Vec<Rectangle> {
    let count = setting.options();
    let chosen = setting.chosen(look.settings);
    let default = setting.chosen(look.defaults);
    let width = layout.scaled(BLOCK_WIDTH);
    let gap = layout.scaled(BLOCK_GAP);
    let blocks = to_f32(rows(count));
    let left = right - blocks * width - (blocks - 1.0).max(0.0) * gap;
    (0..count)
        .map(|option| {
            let paint = match setting.kind() {
                Kind::Colour => {
                    let hue = setting.hue_of(option, look.session);
                    let ink = hue.map_or(OFF_INK, |hue| Ink::Rgb(hue.rgb()));
                    let shown = if option == chosen {
                        1.0
                    } else {
                        SWATCH_RESTING
                    };
                    Paint::shaded(ink, 0, opacity(shown))
                }
                Kind::Choice if option == chosen => Paint::shaded(ON_INK, flash, u8::MAX),
                Kind::Choice if option == default => Paint::ink(DEFAULT_INK),
                Kind::Choice | Kind::Action => Paint::ink(OFF_INK),
            };
            let x = left + to_f32(rows(option)) * (width + gap);
            rectangle([x, top, width, layout.scaled(BLOCK_HEIGHT)], paint)
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/settings/view/instrument.rs"]
mod tests;
