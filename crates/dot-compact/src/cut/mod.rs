mod lasting;
mod lines;
mod middle;
mod outline;
mod thin;

use dot_api::{CUT_OUTPUT_NOTE, CutBounds};

pub use self::lasting::Burst;
use self::thin::Thinning;
use crate::note::NEW_LINE;

pub(crate) const LEAST_WORTH_A_CUT: usize = 4_000;

type Way<'a> = &'a dyn Fn(&str) -> Option<String>;

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    longest_line: usize,
    spares_prose: bool,
    cut_above: usize,
    kept: usize,
    kept_from_the_start: usize,
    outlined: bool,
    thinnings: &'static [Thinning],
}

impl Limits {
    pub(crate) const fn of_contents(bounds: CutBounds) -> Self {
        Self {
            longest_line: 1_000,
            spares_prose: true,
            cut_above: bounds.above,
            kept: bounds.kept,
            kept_from_the_start: bounds.kept / 3 * 2,
            outlined: true,
            thinnings: thin::OF_A_DIFF,
        }
    }

    pub(crate) const fn of_outputs(bounds: CutBounds) -> Self {
        Self {
            longest_line: 400,
            spares_prose: false,
            cut_above: bounds.above,
            kept: bounds.kept,
            kept_from_the_start: bounds.kept / 2,
            outlined: false,
            thinnings: thin::OF_AN_OUTPUT,
        }
    }

    pub(crate) const fn of_a_document(bounds: CutBounds) -> Self {
        Self {
            longest_line: bounds.kept,
            thinnings: &[],
            ..Self::of_outputs(bounds)
        }
    }
}

#[cfg(test)]
pub(crate) const CONTENT: Limits = Limits::of_contents(CutBounds {
    above: 10_000,
    kept: 6_000,
});

#[cfg(test)]
pub(crate) const OTHER: Limits = Limits::of_outputs(CutBounds {
    above: 7_000,
    kept: 3_000,
});

#[cfg(test)]
pub(crate) const ONE_DOCUMENT: Limits = Limits::of_a_document(CutBounds {
    above: 7_000,
    kept: 3_000,
});

pub(crate) fn cut(text: &str, limits: &Limits) -> Option<String> {
    let cuts: [Way<'_>; 2] = [&|text| lines::shorten(text, limits), &|text| {
        middle::cut(text, limits)
    }];
    let thinnings = limits.thinnings.iter().map(|thinning| thinning as Way<'_>);
    let mut shorter: Option<String> = None;
    for way in thinnings.chain(cuts) {
        let current = shorter.as_deref().unwrap_or(text);
        if current.len() <= limits.cut_above {
            break;
        }
        if let Some(next) = way(current).filter(|next| next.len() < current.len()) {
            shorter = Some(next);
        }
    }
    let mut cut = shorter.filter(|cut| cut.len() + LEAST_WORTH_A_CUT <= text.len())?;

    if !cut.ends_with(NEW_LINE) {
        cut.push(NEW_LINE);
    }
    cut.push_str(CUT_OUTPUT_NOTE);
    if text.ends_with(NEW_LINE) {
        cut.push(NEW_LINE);
    }
    Some(cut)
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/cut/mod.rs"]
mod tests;
