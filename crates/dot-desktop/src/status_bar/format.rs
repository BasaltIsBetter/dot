use std::time::Duration;

use dot_api::{Cost, Text, Translator};

const THOUSAND: u64 = 1_000;
const MILLION: u64 = 1_000_000;
const TENTHS_PER_UNIT: u64 = 10;
const TENTH_OF_A_MILLION: u64 = MILLION / TENTHS_PER_UNIT;
const MICRO_DOLLARS_PER_CENT: u64 = 10_000;
const CENTS_PER_DOLLAR: u64 = 100;
const MILLISECONDS_PER_TENTH: u128 = 100;

pub(crate) fn token_count(tokens: u64) -> String {
    if tokens < MILLION {
        return format!("{}k", tokens / THOUSAND);
    }
    let tenths = rounded_quotient(tokens, TENTH_OF_A_MILLION);
    format!("{}.{}M", tenths / TENTHS_PER_UNIT, tenths % TENTHS_PER_UNIT)
}

pub(crate) fn dollars(cost: Cost, translator: &impl Translator) -> String {
    let cents = rounded_quotient(cost.micro_dollars(), MICRO_DOLLARS_PER_CENT);
    format!(
        "{}{}{:02}",
        cents / CENTS_PER_DOLLAR,
        translator.translate(Text::DecimalSeparator),
        cents % CENTS_PER_DOLLAR
    )
}

pub(crate) fn seconds(took: Duration, translator: &impl Translator) -> String {
    let millis = took.as_millis();
    let tenths = millis / MILLISECONDS_PER_TENTH
        + u128::from(millis % MILLISECONDS_PER_TENTH >= MILLISECONDS_PER_TENTH / 2);
    let tenths_per_unit = u128::from(TENTHS_PER_UNIT);
    format!(
        "{}{}{} {}",
        tenths / tenths_per_unit,
        translator.translate(Text::DecimalSeparator),
        tenths % tenths_per_unit,
        translator.translate(Text::UnitSeconds)
    )
}

pub(crate) fn share(part: u64, whole: u64, scale: u16) -> u16 {
    if whole == 0 {
        return 0;
    }
    let scaled = u128::from(part) * u128::from(scale) / u128::from(whole);
    u16::try_from(scaled).map_or(scale, |scaled| scaled.min(scale))
}

fn rounded_quotient(value: u64, divisor: u64) -> u64 {
    value / divisor + u64::from(value % divisor >= divisor / 2)
}

#[cfg(test)]
#[path = "../../../../tests/dot-desktop/status_bar/format.rs"]
mod tests;
