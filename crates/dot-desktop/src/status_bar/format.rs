use dot_api::Cost;

use crate::canvas::columns;

const THOUSAND: u64 = 1_000;
const MILLION: u64 = 1_000_000;
const TENTHS_PER_UNIT: u64 = 10;
const TENTH_OF_A_MILLION: u64 = MILLION / TENTHS_PER_UNIT;
const MICRO_DOLLARS_PER_CENT: u64 = 10_000;
const CENTS_PER_DOLLAR: u64 = 100;
const ELLIPSIS: char = '…';

pub(crate) fn token_count(tokens: u64) -> String {
    if tokens < MILLION {
        return format!("{}k", tokens / THOUSAND);
    }
    let tenths = rounded_quotient(tokens, TENTH_OF_A_MILLION);
    format!("{}.{}M", tenths / TENTHS_PER_UNIT, tenths % TENTHS_PER_UNIT)
}

pub(crate) fn dollars(cost: Cost) -> String {
    let cents = rounded_quotient(cost.micro_dollars(), MICRO_DOLLARS_PER_CENT);
    format!(
        "{}.{:02}",
        cents / CENTS_PER_DOLLAR,
        cents % CENTS_PER_DOLLAR
    )
}

pub(crate) fn share(part: u64, whole: u64, scale: u16) -> u16 {
    if whole == 0 {
        return 0;
    }
    let scaled = u128::from(part) * u128::from(scale) / u128::from(whole);
    u16::try_from(scaled).map_or(scale, |scaled| scaled.min(scale))
}

pub(crate) fn truncated(text: &str, longest: usize) -> String {
    if columns(text) <= longest {
        return text.to_owned();
    }
    if longest == 0 {
        return String::new();
    }
    let kept: String = text.chars().take(longest - 1).collect();
    format!("{kept}{ELLIPSIS}")
}

fn rounded_quotient(value: u64, divisor: u64) -> u64 {
    value / divisor + u64::from(value % divisor >= divisor / 2)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/format.rs"]
mod tests;
