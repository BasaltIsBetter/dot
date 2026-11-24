mod action;
mod draw;
mod instrument;

use std::time::{Duration, Instant};

use dot_api::{Hue, Settings, Translator};
use dot_transcript::{Ink, Line, Motion, Span, opacity, rows, to_f32};

use crate::canvas::{Layout, Paint, Pane, Rectangle, TextSize};
use crate::settings::Waiting;
use crate::settings::catalog::{ENTRIES, Entry, Kind, Setting};
use crate::settings::reel::{Reel, is_changed};

use self::draw::{
    HAIRLINE, HINT_SPACE, ROW, below, interpolated, masked, offsets, pane, rectangle, ring,
    width_of,
};

pub(crate) use self::action::Actions;
pub(crate) use self::draw::{Frame, inside};
pub(crate) use self::instrument::{band, hint, instrument};

const OPTION_GAP: &str = "  ";
const UNDERLINE_FROM_BOTTOM: f32 = 6.0;
const SWATCH: f32 = 10.0;
const SWATCH_GAP: f32 = 6.0;
const SWATCH_RING: f32 = 2.0;
const SWATCH_RESTING: f64 = 0.4;
const SWATCH_HOVERED: f64 = 0.8;
const SWATCH_TO_NAME: f32 = 14.0;
const MARK_FROM_RIGHT: f32 = 16.0;
const MARK_WIDTH: f32 = 12.0;
const RULE_GAP: f32 = 12.0;
const OF_THE_SESSION: usize = 0;
const FILTERED: f64 = 0.22;
const COOLING: Motion = Motion::landing(Duration::from_millis(1_200));
const FOCUSED_INK: Ink = Ink::Full;
const RESTING_INK: Ink = Ink::Secondary;
const OTHER_OPTION_INK: Ink = Ink::Faint;
const HOVERED_OPTION_INK: Ink = Ink::Secondary;
const SECTION_INK: Ink = Ink::Receding;
const RULE_INK: Ink = Ink::Trace;
const MARK_INK: Ink = Ink::Secondary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Focus(usize),
    Choose(Setting, usize),
}

#[derive(Debug, Default)]
pub(crate) struct Drawn {
    pub(crate) panes: Vec<Pane>,
    pub(crate) marks: Vec<Rectangle>,
    pub(crate) hits: Vec<([f32; 4], Hit)>,
}

pub(crate) struct Look<'a, T> {
    pub(crate) settings: &'a Settings,
    pub(crate) defaults: &'a Settings,
    pub(crate) session: Hue,
    pub(crate) translator: &'a T,
    pub(crate) hover: Option<[f32; 2]>,
    pub(crate) changed: Option<(Setting, Instant)>,
    pub(crate) actions: &'a Actions,
    pub(crate) waiting: Waiting,
}

struct Row<'a> {
    layout: &'a Layout,
    frame: &'a Frame,
    top: f32,
    alpha: f64,
    ink: Ink,
    heat: u8,
}

pub(crate) fn entries<T: Translator>(
    reel: &Reel,
    look: &Look<'_, T>,
    (layout, frame): (&Layout, &Frame),
    now: Instant,
) -> Drawn {
    let mut drawn = Drawn::default();
    let offsets = offsets(layout);
    let position = f64::from(reel.shift.position(now));
    let shown = interpolated(&offsets, position);
    let focus = reel.focus_index();
    let height = layout.scaled(ROW);

    for (place, entry) in ENTRIES.iter().enumerate() {
        let top = (frame.band_top + offsets[place] - shown
            + layout.scaled(HINT_SPACE) * below(place, position))
        .round();
        if top + height < 0.0 || top > frame.floor {
            continue;
        }
        let mut alpha = masked(top + height / 2.0, frame.floor);
        if !reel.matches(place, look.translator) {
            alpha *= FILTERED;
        }
        let bounds = [frame.left, top, frame.width, height];
        match entry {
            Entry::Section(text) => {
                let name = look.translator.translate(*text);
                section(&mut drawn, &name, (layout, frame), (top, alpha));
            }
            Entry::Setting(setting) => {
                let focused = place == focus;
                let hovered = look.hover.is_some_and(|point| inside(point, bounds));
                drawn.hits.push((bounds, Hit::Focus(place)));
                let row = Row {
                    layout,
                    frame,
                    top,
                    alpha,
                    ink: if focused || hovered {
                        FOCUSED_INK
                    } else {
                        RESTING_INK
                    },
                    heat: heat_of(*setting, look.changed, now),
                };
                setting_row(&mut drawn, *setting, focused, look, &row);
            }
        }
    }
    drawn
}

fn section(
    drawn: &mut Drawn,
    name: &str,
    (layout, frame): (&Layout, &Frame),
    (top, alpha): (f32, f64),
) {
    let height = layout.scaled(ROW);
    let line = Line::new(0, vec![Span::new(name, SECTION_INK)]).faded(alpha);
    let text_top = layout.text_top(TextSize::Small, top, height);
    drawn
        .panes
        .push(pane(frame.left, TextSize::Small, text_top, line));
    let start =
        frame.left + layout.advance_of(TextSize::Small) * width_of(name) + layout.scaled(RULE_GAP);
    if start < frame.right() {
        let bounds = [
            start,
            (top + height / 2.0).round(),
            frame.right() - start,
            HAIRLINE,
        ];
        drawn.marks.push(rectangle(
            bounds,
            Paint::shaded(RULE_INK, 0, opacity(alpha)),
        ));
    }
}

fn setting_row<T: Translator>(
    drawn: &mut Drawn,
    setting: Setting,
    focused: bool,
    look: &Look<'_, T>,
    row: &Row<'_>,
) {
    let layout = row.layout;
    let height = layout.scaled(ROW);
    let text_top = layout.text_top(TextSize::Regular, row.top, height);
    let label = look.translator.translate(setting.label());
    let line = Line::new(0, vec![Span::new(label.as_ref(), row.ink)]).faded(row.alpha);
    drawn
        .panes
        .push(pane(row.frame.left, TextSize::Regular, text_top, line));

    let options: Vec<usize> = if focused {
        (0..setting.options()).collect()
    } else {
        vec![setting.chosen(look.settings)]
    };
    match setting.kind() {
        Kind::Choice => choices(drawn, setting, (&options, focused), look, row),
        Kind::Colour => colours(drawn, setting, (&options, focused), look, row),
        Kind::Action => action::action(drawn, (setting, focused), look, row),
    }

    if is_changed(setting, look.settings, look.defaults) {
        let bounds = [
            row.frame.right() + layout.scaled(MARK_FROM_RIGHT),
            (row.top + height / 2.0).round(),
            layout.scaled(MARK_WIDTH),
            HAIRLINE,
        ];
        drawn.marks.push(rectangle(
            bounds,
            Paint::shaded(MARK_INK, 0, opacity(row.alpha)),
        ));
    }
}

fn choices<T: Translator>(
    drawn: &mut Drawn,
    setting: Setting,
    (options, focused): (&[usize], bool),
    look: &Look<'_, T>,
    row: &Row<'_>,
) {
    let layout = row.layout;
    let height = layout.scaled(ROW);
    let advance = layout.advance_of(TextSize::Regular);
    let chosen = setting.chosen(look.settings);
    let names: Vec<String> = options
        .iter()
        .map(|&option| setting.option_name(option, look.translator))
        .collect();
    let gaps = to_f32(rows(names.len().saturating_sub(1)));
    let total = names.iter().map(|name| width_of(name)).sum::<f32>() + width_of(OPTION_GAP) * gaps;
    let start = row.frame.right() - total * advance;

    let mut x = start;
    let mut spans = Vec::new();
    for (&option, name) in options.iter().zip(&names) {
        let current = option == chosen;
        let width = width_of(name) * advance;
        let bounds = [x, row.top, width, height];
        let ink = match (focused && !current, hovers(look, bounds)) {
            (false, _) => row.ink,
            (true, false) => OTHER_OPTION_INK,
            (true, true) => HOVERED_OPTION_INK,
        };
        let heat = if current { row.heat } else { 0 };
        if current && focused {
            let underline = [
                x,
                row.top + height - layout.scaled(UNDERLINE_FROM_BOTTOM),
                width,
                HAIRLINE,
            ];
            drawn.marks.push(rectangle(
                underline,
                Paint::shaded(ink, heat, opacity(row.alpha)),
            ));
        }
        drawn.hits.push((bounds, Hit::Choose(setting, option)));
        spans.push(Span::new(name.as_str(), ink).heated(heat));
        spans.push(Span::new(OPTION_GAP, ink));
        x += width + width_of(OPTION_GAP) * advance;
    }
    spans.pop();
    let text_top = layout.text_top(TextSize::Regular, row.top, height);
    let line = Line::new(0, spans).faded(row.alpha);
    drawn
        .panes
        .push(pane(start, TextSize::Regular, text_top, line));
}

fn colours<T: Translator>(
    drawn: &mut Drawn,
    setting: Setting,
    (options, focused): (&[usize], bool),
    look: &Look<'_, T>,
    row: &Row<'_>,
) {
    let layout = row.layout;
    let height = layout.scaled(ROW);
    let chosen = setting.chosen(look.settings);
    let name = setting.option_name(chosen, look.translator);
    let name_width = width_of(&name) * layout.advance_of(TextSize::Regular);
    let swatch = layout.scaled(SWATCH);
    let gap = layout.scaled(SWATCH_GAP);
    let count = to_f32(rows(options.len()));
    let mut x = row.frame.right()
        - name_width
        - layout.scaled(SWATCH_TO_NAME)
        - count * swatch
        - (count - 1.0).max(0.0) * gap;
    let swatch_top = (row.top + (height - swatch) / 2.0).round();

    for &option in options {
        let Some(hue) = setting.hue_of(option, look.session) else {
            continue;
        };
        let current = option == chosen;
        let hit = [x, row.top, swatch + gap, height];
        let shown = row.alpha
            * match (current, hovers(look, [x, swatch_top, swatch, swatch])) {
                (true, _) => 1.0,
                (false, true) => SWATCH_HOVERED,
                (false, false) => SWATCH_RESTING,
            };
        let paint = Paint::shaded(Ink::Rgb(hue.rgb()), 0, opacity(shown));
        let inset = if option == OF_THE_SESSION {
            HAIRLINE
        } else {
            0.0
        };
        drawn.marks.push(rectangle(
            [
                x + inset,
                swatch_top + inset,
                swatch - 2.0 * inset,
                swatch - 2.0 * inset,
            ],
            paint,
        ));
        if current && focused {
            let ring_paint = Paint::shaded(FOCUSED_INK, 0, opacity(row.alpha));
            drawn.marks.extend(ring(
                [x, swatch_top, swatch],
                layout.scaled(SWATCH_RING),
                ring_paint,
            ));
        }
        drawn.hits.push((hit, Hit::Choose(setting, option)));
        x += swatch + gap;
    }
    let text_top = layout.text_top(TextSize::Regular, row.top, height);
    let line =
        Line::new(0, vec![Span::new(name.as_str(), row.ink).heated(row.heat)]).faded(row.alpha);
    drawn.panes.push(pane(
        row.frame.right() - name_width,
        TextSize::Regular,
        text_top,
        line,
    ));
}

fn hovers<T: Translator>(look: &Look<'_, T>, bounds: [f32; 4]) -> bool {
    look.hover.is_some_and(|point| inside(point, bounds))
}

fn heat_of(setting: Setting, changed: Option<(Setting, Instant)>, now: Instant) -> u8 {
    changed
        .filter(|(changed, _)| *changed == setting)
        .and_then(|(_, at)| COOLING.eased(now.saturating_duration_since(at)))
        .map_or(0, |eased| opacity(1.0 - eased))
}

pub(crate) fn is_cooling(changed: Option<(Setting, Instant)>, now: Instant) -> bool {
    changed.is_some_and(|(_, at)| COOLING.lasts(now.saturating_duration_since(at)))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/settings/view/mod.rs"]
pub(crate) mod tests;
