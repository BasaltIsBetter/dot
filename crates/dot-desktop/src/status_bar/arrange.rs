use crate::canvas::{Gauge, Strip};
use crate::status_bar::view::{Field, GAP, Item, Zone};
use dot_transcript::{Ink, Line, Span};

const MIDDLE_PADDING: u16 = 2;
const SHORTEST_STATE: u16 = 6;
const BLANK: &str = " ";
const BLANK_INK: Ink = Ink::Full;

struct Zones {
    left: Vec<Item>,
    state: Option<Item>,
    middle: Vec<Item>,
    right: Vec<Item>,
}

pub(crate) fn arrange(mut items: Vec<Item>, columns: u16, middle_shown: bool) -> Strip {
    if !middle_shown {
        items.retain(|item| item.field.zone() != Zone::Middle);
    }
    loop {
        if let Some(strip) = laid_out(&items, columns) {
            return strip;
        }
        let least_needed = items
            .iter()
            .enumerate()
            .max_by_key(|(_, item)| item.field)
            .map(|(index, _)| index);
        let Some(least_needed) = least_needed else {
            return Strip::default();
        };
        items.remove(least_needed);
    }
}

fn laid_out(items: &[Item], columns: u16) -> Option<Strip> {
    let mut zones = Zones::of(items.to_vec());
    let left_end = width_of(&zones.left);
    let middle_width = if zones.middle.is_empty() {
        0
    } else {
        width_of(&zones.middle).checked_add(2 * MIDDLE_PADDING)?
    };
    let right_width = width_of(&zones.right);
    let middle_start = columns.checked_sub(middle_width)? / 2;
    let right_start = columns.checked_sub(right_width)?;
    let middle_end = middle_start + middle_width;

    let left_limit = if middle_width == 0 {
        right_start.checked_sub(GAP)?
    } else {
        middle_start.checked_sub(GAP)?
    };
    let middle_fits = middle_width == 0 || middle_end.saturating_add(GAP) <= right_start;
    if left_end > left_limit || !middle_fits {
        return None;
    }
    let state_room = left_limit.saturating_sub(left_end + GAP);
    zones.state = zones
        .state
        .take()
        .filter(|_| state_room >= SHORTEST_STATE)
        .map(|mut state| {
            state.cut_to(state_room);
            state
        });
    Some(placed(zones, [middle_start, middle_width, right_start]))
}

fn placed(zones: Zones, [middle_start, middle_width, right_start]: [u16; 3]) -> Strip {
    let mut strip = Strip::default();
    let mut cursor = 0;
    let mut column = 0;
    for item in zones.left.into_iter().chain(zones.state) {
        cursor = place(&mut strip, item, column, cursor);
        column = cursor + GAP;
    }
    if middle_width > 0 {
        strip.rules = vec![middle_start, middle_start + middle_width];
        column = middle_start + MIDDLE_PADDING;
        for item in zones.middle {
            cursor = place(&mut strip, item, column, cursor);
            column = cursor + GAP;
        }
    }
    column = right_start;
    for item in zones.right {
        cursor = place(&mut strip, item, column, cursor);
        column = cursor + GAP;
    }
    strip
}

impl Zones {
    fn of(items: Vec<Item>) -> Self {
        let mut zones = Self {
            left: Vec::new(),
            state: None,
            middle: Vec::new(),
            right: Vec::new(),
        };
        for item in items {
            match (item.field, item.field.zone()) {
                (Field::State, _) => zones.state = Some(item),
                (_, Zone::Left) => zones.left.push(item),
                (_, Zone::Middle) => zones.middle.push(item),
                (_, Zone::Right) => zones.right.push(item),
            }
        }
        zones
    }
}

fn place(strip: &mut Strip, item: Item, column: u16, cursor: u16) -> u16 {
    let width = item.width();
    if let Some(target) = item.target {
        strip
            .targets
            .push((target, [column, column.saturating_add(width)]));
    }
    if let Some(icon) = item.icon {
        strip.icons.push((column, icon));
    }
    if let Some((fill_per_mille, tint)) = item.gauge {
        strip.gauge = Some(Gauge {
            column,
            fill_per_mille,
            tint,
        });
    }
    pad(&mut strip.text, column.saturating_sub(cursor));
    strip.text.spans.extend(item.spans);
    column.saturating_add(width)
}

fn width_of(items: &[Item]) -> u16 {
    let gaps = u16::try_from(items.len().saturating_sub(1)).unwrap_or(u16::MAX);
    items.iter().fold(GAP.saturating_mul(gaps), |cells, item| {
        cells.saturating_add(item.width())
    })
}

fn pad(line: &mut Line, cells: u16) {
    if cells > 0 {
        line.spans
            .push(Span::new(BLANK.repeat(usize::from(cells)), BLANK_INK));
    }
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/arrange.rs"]
mod tests;
