use core::cmp::Ordering;

use crate::util::{abs_i128_to_u128, pow10_u128, signed_from_abs};
use crate::{Decimal, SCALE};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoundingStrategy {
    ToZero,
    AwayFromZero,
    MidpointTowardZero,
    MidpointAwayFromZero,
}

pub(crate) fn scale_mantissa(
    mantissa: i128,
    scale: u32,
    strategy: RoundingStrategy,
) -> Option<Decimal> {
    let negative = mantissa < 0;
    let abs = abs_i128_to_u128(mantissa);

    let raw_abs = match scale.cmp(&SCALE) {
        Ordering::Equal => abs,
        Ordering::Less => {
            let factor = pow10_u128(SCALE - scale)?;
            abs.checked_mul(factor)?
        }
        Ordering::Greater => {
            let diff = scale - SCALE;
            let Some(factor) = pow10_u128(diff) else {
                return Some(Decimal::ZERO);
            };
            let q = abs / factor;
            let r = abs % factor;
            apply_rounding_abs(q, r, factor, strategy)?
        }
    };

    signed_from_abs(raw_abs, negative).map(Decimal::from_raw)
}

pub(crate) fn round_to_unit(raw: i128, unit: i128, strategy: RoundingStrategy) -> Decimal {
    debug_assert!(unit > 0);

    let negative = raw < 0;
    let abs = abs_i128_to_u128(raw);
    let unit = unit as u128;
    let q = abs / unit;
    let r = abs % unit;
    let rounded = apply_rounding_abs(q, r, unit, strategy)
        .and_then(|q| q.checked_mul(unit))
        .and_then(|raw_abs| signed_from_abs(raw_abs, negative))
        .expect("Decimal rounding overflow");

    Decimal::from_raw(rounded)
}

fn apply_rounding_abs(q: u128, r: u128, unit: u128, strategy: RoundingStrategy) -> Option<u128> {
    let increment = match strategy {
        RoundingStrategy::ToZero => false,
        RoundingStrategy::AwayFromZero => r != 0,
        RoundingStrategy::MidpointTowardZero => r.checked_mul(2)? > unit,
        RoundingStrategy::MidpointAwayFromZero => r.checked_mul(2)? >= unit,
    };

    if increment {
        q.checked_add(1)
    } else {
        Some(q)
    }
}
