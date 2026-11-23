mod bar;

use std::ops::Range;
use std::time::Instant;

use dot_api::Translator;
use dot_transcript::{
    Ink, Line, NEW_LINE, Placed, Span, TextWidth, cells, code_inks, index, opacity, rows, shade,
    to_f32,
};

use crate::canvas::{Bounds, Layout, Pane, Rectangle, Rgba, Scene, TextSize};
use crate::conversation::{Caret, Stage};
use crate::editor::Editor;
use crate::editor::view::bar::{Bar, Hit};

pub(crate) use self::bar::activity;

const TREE_WIDTH: f32 = 200.0;
const BAR_TOP: f32 = 18.0;
const BAR_BELOW: f32 = 14.0;
const NUMBERS_LEFT: f32 = 16.0;
const NUMBERS_RIGHT: f32 = 14.0;
const CODE_LEFT: f32 = 14.0;
const CODE_RIGHT: f32 = 24.0;
const MARK: &str = "── ";
const NO_MARK: &str = "   ";
const CARET: &str = "_";
const TAB: char = '\t';
const TAB_SPACES: &str = "    ";
const WASH: f64 = 0.38;
const NARROWEST: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pressed {
    Nothing,
    Placed,
    Changed,
    Open(String),
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    tree: Option<f32>,
    pane: f32,
    bar_top: f32,
    code_top: f32,
    numbers: f32,
    digits: usize,
    rule: f32,
    code: f32,
    right: f32,
    bottom: f32,
    row: f32,
    advance: f32,
}

impl Frame {
    fn of(layout: &Layout, lines: usize) -> Self {
        let tree = layout
            .room()
            .has_gutter()
            .then(|| layout.scaled(TREE_WIDTH));
        let pane = tree.unwrap_or(0.0);
        let bar_top = layout.scaled(BAR_TOP);
        let below_bar = layout.line_height_of(TextSize::Small) + layout.scaled(BAR_BELOW);
        let advance = layout.advance();
        let digits = lines.checked_ilog10().map_or(1, |power| power as usize + 1);
        let numbers = pane + layout.scaled(NUMBERS_LEFT);
        let rule =
            numbers + width_of(columns_of(MARK) + digits, advance) + layout.scaled(NUMBERS_RIGHT);
        Self {
            tree,
            pane,
            bar_top,
            code_top: bar_top + below_bar,
            numbers,
            digits,
            rule: rule.round(),
            code: rule.round() + layout.scaled(CODE_LEFT),
            right: layout.width() - layout.scaled(CODE_RIGHT),
            bottom: layout.strip_top(),
            row: layout.line_height(),
            advance,
        }
    }

    fn width(&self) -> TextWidth {
        let columns = ((self.right - self.code) / self.advance).floor().max(0.0);
        TextWidth::new(index(f64::from(columns)).max(NARROWEST))
    }

    fn visible(&self) -> usize {
        index(f64::from(
            ((self.bottom - self.code_top) / self.row).floor().max(1.0),
        ))
    }

    fn row_top(&self, shown: usize) -> f32 {
        (self.code_top + to_f32(rows(shown)) * self.row).round()
    }

    fn x_of(&self, column: usize) -> f32 {
        self.code + width_of(column, self.advance)
    }
}

impl Editor {
    pub(crate) fn scene(
        &mut self,
        layout: &Layout,
        now: Instant,
        (pointer, caret): (Option<[f32; 2]>, Caret),
        translator: &impl Translator,
    ) -> Scene {
        let frame = self.frame(layout);
        self.fit(frame.width(), frame.visible());
        let bar = Bar::of(&frame, layout, translator);
        let mut grounds = vec![self.caret_band(&frame)];
        grounds.extend(self.selection(&frame));
        let (numbers, code) = self.code_lines(&frame, caret, now);
        let mut panes = vec![
            pane(frame.numbers, &frame, numbers),
            pane(frame.code, &frame, code),
            bar.pane(self, &frame, pointer, translator),
        ];
        let rule = |x: f32, top: f32| Rectangle {
            left: x,
            top,
            width: layout.scaled(1.0).max(1.0),
            height: frame.bottom - top,
            color: Rgba::ink(Ink::Ground),
        };
        let mut marks = vec![rule(frame.rule, frame.code_top)];
        if let Some(tree) = frame.tree {
            panes.extend(bar.tree(self, layout, &frame));
            marks.push(rule(tree, 0.0));
        }
        let mut scene = Scene {
            panes,
            grounds,
            marks,
            ..Scene::default()
        };
        self.fade_in(&mut scene, Stage::of(layout).middle(), now);
        scene
    }

    pub(crate) fn press(
        &mut self,
        [x, y]: [f32; 2],
        (extend, now): (bool, Instant),
        layout: &Layout,
        translator: &impl Translator,
    ) -> Pressed {
        let frame = self.frame(layout);
        match Bar::of(&frame, layout, translator).hit(self, [x, y], &frame) {
            Some(Hit::Back) => return changed(self.step(true, now)),
            Some(Hit::Forward) => return changed(self.step(false, now)),
            Some(Hit::Close) => return Pressed::Close,
            Some(Hit::Open(path)) => return Pressed::Open(path),
            None => {}
        }
        if x < frame.pane || y < frame.code_top || y >= frame.bottom {
            return Pressed::Nothing;
        }
        let row = self.top + index(f64::from(((y - frame.code_top) / frame.row).floor()));
        if row >= self.buffer.rows().len() {
            return Pressed::Nothing;
        }
        let column = index(f64::from(
            ((x - frame.code) / frame.advance).round().max(0.0),
        ));
        self.buffer.place(row, column, extend);
        Pressed::Placed
    }

    pub(crate) fn caret_area(&self, layout: &Layout) -> Bounds {
        let frame = self.frame(layout);
        let (row, column) = self.buffer.caret_cell();
        Bounds {
            left: frame.x_of(column),
            top: frame.row_top(row.saturating_sub(self.top)),
            width: frame.advance,
            height: frame.row,
        }
    }

    fn frame(&self, layout: &Layout) -> Frame {
        Frame::of(layout, self.buffer.text().split(NEW_LINE).count())
    }

    fn code_lines(&self, frame: &Frame, caret: Caret, now: Instant) -> (Vec<Placed>, Vec<Placed>) {
        let text = self.buffer.text();
        let all = self.buffer.rows();
        let end = self.top.saturating_add(self.visible).min(all.len());
        let first = all.get(self.top).map_or(0, |row| row.start);
        let mut number = text[..first].matches(NEW_LINE).count();
        let (mut numbers, mut code) = (Vec::new(), Vec::new());
        for (shown, row) in all[self.top..end].iter().enumerate() {
            let top = frame.row_top(shown);
            let line_start = text[..row.start].rfind(NEW_LINE).map_or(0, |at| at + 1);
            let line_end = text[row.start..]
                .find(NEW_LINE)
                .map_or(text.len(), |at| row.start + at);
            if line_start == row.start {
                let line = self.number(number, &text[line_start..line_end], frame.digits);
                numbers.push(Placed {
                    top,
                    line,
                    clip: None,
                });
                number += 1;
            }
            let spans = self.row_spans(row.clone(), line_start..line_end, now);
            code.push(Placed {
                top,
                line: Line::new(0, spans),
                clip: None,
            });
        }
        let ink = match caret {
            Caret::Lit => Some(Ink::Live),
            Caret::Receded => Some(Ink::Receding),
            Caret::Dark => None,
        };
        if let Some(caret) = ink.and_then(|ink| self.caret_line(frame, ink)) {
            code.push(caret);
        }
        (numbers, code)
    }

    fn number(&self, index: usize, line: &str, digits: usize) -> Line {
        let mine = self.mine.changed.get(index).copied().unwrap_or(false);
        let by_claude = !self.versions.created() && self.versions.by_claude(line);
        let (mark, ink, heat) = match (mine, by_claude) {
            (true, _) => (MARK, Ink::Full, u8::MAX),
            (false, true) => (MARK, Ink::Full, 0),
            (false, false) => (NO_MARK, Ink::Faint, 0),
        };
        let number = format!("{mark}{:>digits$}", index + 1);
        Line::new(0, vec![Span::new(number, ink).heated(heat)])
    }

    fn row_spans(&self, row: Range<usize>, line: Range<usize>, now: Instant) -> Vec<Span> {
        let text = self.buffer.text();
        let inks: Vec<(Range<usize>, Ink)> = code_inks(&text[line.clone()], self.language)
            .into_iter()
            .map(|(range, ink)| (line.start + range.start..line.start + range.end, ink))
            .collect();
        let heats = self.buffer.heat(row.clone(), now);
        let mut cuts = vec![row.start, row.end];
        cuts.extend(inks.iter().flat_map(|(range, _)| [range.start, range.end]));
        cuts.extend(heats.iter().flat_map(|(range, _)| [range.start, range.end]));
        cuts.retain(|cut| (row.start..=row.end).contains(cut));
        cuts.sort_unstable();
        cuts.dedup();
        cuts.windows(2)
            .map(|pair| {
                let at = pair[0];
                let ink = inks
                    .iter()
                    .find(|(range, _)| range.contains(&at))
                    .map_or(Ink::Full, |(_, ink)| *ink);
                let heat = heats
                    .iter()
                    .find(|(range, _)| range.contains(&at))
                    .map_or(0, |(_, heat)| *heat);
                Span::new(text[at..pair[1]].replace(TAB, TAB_SPACES), ink).heated(heat)
            })
            .collect()
    }

    fn caret_line(&self, frame: &Frame, ink: Ink) -> Option<Placed> {
        let (row, column) = self.buffer.caret_cell();
        let shown = row
            .checked_sub(self.top)
            .filter(|&shown| shown < self.visible)?;
        let line = Line::new(
            u16::try_from(column).unwrap_or(u16::MAX),
            vec![Span::new(CARET, ink)],
        );
        Some(Placed {
            top: frame.row_top(shown),
            line,
            clip: None,
        })
    }

    fn caret_band(&self, frame: &Frame) -> Rectangle {
        let (row, _) = self.buffer.caret_cell();
        let shown = row.saturating_sub(self.top).min(self.visible);
        Rectangle {
            left: frame.rule,
            top: frame.row_top(shown),
            width: frame.right + frame.advance - frame.rule,
            height: frame.row,
            color: Rgba::ink(Ink::Ground),
        }
    }

    fn selection(&self, frame: &Frame) -> Vec<Rectangle> {
        let Some(selected) = self.buffer.selection() else {
            return Vec::new();
        };
        let text = self.buffer.text();
        let color = Rgba {
            rgb: shade(Ink::Live, 0, opacity(WASH)),
            alpha: u8::MAX,
        };
        let all = self.buffer.rows();
        let end = self.top.saturating_add(self.visible).min(all.len());
        all[self.top..end]
            .iter()
            .enumerate()
            .filter_map(|(shown, row)| {
                let start = selected.start.max(row.start);
                let stop = selected.end.min(row.end);
                let through_a_break = selected.start <= row.end && selected.end > row.end;
                (start < stop || through_a_break).then(|| {
                    let from = columns_of(&text[row.start..start]);
                    let to = columns_of(&text[row.start..stop]) + usize::from(through_a_break);
                    let left = frame.x_of(from);
                    Rectangle {
                        left,
                        top: frame.row_top(shown),
                        width: frame.x_of(to) - left,
                        height: frame.row,
                        color,
                    }
                })
            })
            .collect()
    }
}

fn changed(stepped: bool) -> Pressed {
    if stepped {
        Pressed::Changed
    } else {
        Pressed::Nothing
    }
}

fn pane(left: f32, frame: &Frame, lines: Vec<Placed>) -> Pane {
    Pane {
        left,
        size: TextSize::Regular,
        top: frame.code_top,
        bottom: frame.bottom,
        lines,
    }
}

fn columns_of(text: &str) -> usize {
    text.chars().map(cells).sum()
}

fn width_of(columns: usize, advance: f32) -> f32 {
    to_f32(rows(columns)) * advance
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/editor/view/mod.rs"]
mod tests;
