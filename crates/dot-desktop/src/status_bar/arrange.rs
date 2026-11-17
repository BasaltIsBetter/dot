use crate::canvas::{Gauge, Ink, Line, Span, Strip};
use crate::status_bar::view::Module;

const GAP: u16 = 4;
const SEPARATOR_OFFSET: u16 = 2;
const BLANK: &str = " ";
const BLANK_INK: Ink = Ink::Full;

pub(crate) fn arrange(mut modules: Vec<Module>, columns: u16) -> Strip {
    while total_width(&modules) > columns {
        let least_needed = modules
            .iter()
            .enumerate()
            .max_by_key(|(_, module)| module.field)
            .map(|(index, _)| index);
        let Some(least_needed) = least_needed else {
            break;
        };
        modules.remove(least_needed);
    }
    modules.sort_by_key(|module| module.field.group());

    let mut strip = Strip::default();
    let mut cursor: u16 = 0;
    let mut column = columns.saturating_sub(total_width(&modules)) / 2;
    let mut group = None;
    for module in modules {
        let module_group = module.field.group();
        if let Some(group) = group {
            let separator = cursor.saturating_add(SEPARATOR_OFFSET);
            if group == module_group {
                strip.rules.push(separator);
            } else {
                strip.dividers.push(separator);
            }
            column = cursor.saturating_add(GAP);
        }
        group = Some(module_group);
        cursor = place(&mut strip, module, column, cursor);
    }
    strip
}

fn place(strip: &mut Strip, module: Module, column: u16, cursor: u16) -> u16 {
    let width = module.width();
    if let Some(icon) = module.icon {
        strip.icons.push((column, icon));
    }
    if let Some(gauge) = module.gauge {
        strip.gauge = Some(Gauge {
            column: column.saturating_add(gauge.column),
            ..gauge
        });
    }
    pad(&mut strip.text, column.saturating_sub(cursor));
    strip.text.spans.extend(module.spans);
    column.saturating_add(width)
}

fn total_width(modules: &[Module]) -> u16 {
    let separators = u16::try_from(modules.len().saturating_sub(1)).unwrap_or(u16::MAX);
    modules
        .iter()
        .fold(GAP.saturating_mul(separators), |cells, module| {
            cells.saturating_add(module.width())
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
