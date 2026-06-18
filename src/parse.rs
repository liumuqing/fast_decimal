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
