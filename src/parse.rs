use core::str::FromStr;

use crate::util::{signed_from_abs, SCALE_FACTOR_U128};
use crate::{Decimal, DecimalError, SCALE};

impl FromStr for Decimal {
    type Err = DecimalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_decimal(s)
    }
}

pub(crate) fn parse_decimal(s: &str) -> Result<Decimal, DecimalError> {
    if s.is_empty() {
        return Err(DecimalError::InvalidSyntax);
    }

    if let Some((mantissa, exponent)) = s.split_once(['e', 'E']) {
        return parse_scientific(mantissa, exponent);
    }

    let bytes = s.as_bytes();
    let mut index = 0usize;
    let mut negative = false;

    match bytes[0] {
        b'-' => {
            negative = true;
            index = 1;
        }
        b'+' => index = 1,
        _ => {}
    }

    if index == bytes.len() {
        return Err(DecimalError::InvalidSyntax);
    }

    let mut int: u128 = 0;
    let mut frac: u128 = 0;
    let mut frac_len = 0u32;
    let mut round_digit: Option<u8> = None;
    let mut saw_digit = false;
    let mut saw_dot = false;

    while index < bytes.len() {
        let b = bytes[index];
        index += 1;

        match b {
            b'_' => continue,
            b'.' if !saw_dot => {
                saw_dot = true;
            }
            b'0'..=b'9' => {
                saw_digit = true;
                let digit = b - b'0';

                if saw_dot {
                    if frac_len < SCALE {
                        frac = frac
                            .checked_mul(10)
                            .and_then(|v| v.checked_add(digit as u128))
                            .ok_or(DecimalError::Overflow)?;
                        frac_len += 1;
                    } else if round_digit.is_none() {
                        round_digit = Some(digit);
                    }
                } else {
                    int = int
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(digit as u128))
                        .ok_or(DecimalError::Overflow)?;
                }
            }
            _ => return Err(DecimalError::InvalidSyntax),
        }
    }

    if !saw_digit {
        return Err(DecimalError::InvalidSyntax);
    }

    for _ in frac_len..SCALE {
        frac = frac.checked_mul(10).ok_or(DecimalError::Overflow)?;
    }

    let mut raw_abs = int
        .checked_mul(SCALE_FACTOR_U128)
        .and_then(|v| v.checked_add(frac))
        .ok_or(DecimalError::Overflow)?;

    if matches!(round_digit, Some(5..=9)) {
        raw_abs = raw_abs.checked_add(1).ok_or(DecimalError::Overflow)?;
    }

    signed_from_abs(raw_abs, negative)
        .map(Decimal::from_raw)
        .ok_or(DecimalError::Overflow)
}

// Scan significant digits directly: no floating point and no allocation
// proportional to the exponent. Round once, after applying the exponent.
fn parse_scientific(mantissa: &str, exponent: &str) -> Result<Decimal, DecimalError> {
    let (negative, mantissa) = strip_sign(mantissa);
    let (exp_negative, exponent) = strip_sign(exponent);
    if exponent.is_empty() || !exponent.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DecimalError::InvalidSyntax);
    }
    // Saturation permits tiny values to round to zero for huge negative
    // exponents. Huge positive nonzero values still report overflow.
    let exp = exponent.bytes().fold(0i64, |n, b| {
        n.saturating_mul(10).saturating_add(i64::from(b - b'0'))
    });
    let exp = if exp_negative { -exp } else { exp };
    let mut saw_dot = false;
    let mut digits = 0i64;
    let mut integer_digits = 0i64;
    let mut leading_zeros = 0usize;
    let mut nonzero = false;
    for b in mantissa.bytes() {
        match b {
            b'_' => {}
            b'.' if !saw_dot => saw_dot = true,
            b'0'..=b'9' => {
                digits += 1;
                if !saw_dot {
                    integer_digits += 1;
                }
                if b != b'0' {
                    nonzero = true;
                }
                if !nonzero {
                    leading_zeros += 1;
                }
            }
            _ => return Err(DecimalError::InvalidSyntax),
        }
    }
    if digits == 0 {
        return Err(DecimalError::InvalidSyntax);
    }
    if !nonzero {
        return Ok(Decimal::ZERO);
    }
    let scaled_digits =
        (integer_digits - leading_zeros as i64 + i64::from(SCALE)).saturating_add(exp);
    if scaled_digits > 39 {
        return Err(DecimalError::Overflow);
    }
    if scaled_digits < 0 {
        return Ok(Decimal::ZERO);
    }

    let mut significant = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .skip(leading_zeros);
    let mut raw_abs = 0u128;
    for _ in 0..scaled_digits {
        let digit = significant.next().map_or(0, |b| b - b'0');
        raw_abs = raw_abs
            .checked_mul(10)
            .and_then(|n| n.checked_add(u128::from(digit)))
            .ok_or(DecimalError::Overflow)?;
    }
    if significant.next().is_some_and(|b| b >= b'5') {
        raw_abs = raw_abs.checked_add(1).ok_or(DecimalError::Overflow)?;
    }
    signed_from_abs(raw_abs, negative)
        .map(Decimal::from_raw)
        .ok_or(DecimalError::Overflow)
}

fn strip_sign(s: &str) -> (bool, &str) {
    if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    }
}
