use crate::SCALE_FACTOR;

pub(crate) const SCALE_FACTOR_U128: u128 = SCALE_FACTOR as u128;
const I128_MIN_ABS: u128 = 1u128 << 127;

pub(crate) fn abs_i128_to_u128(value: i128) -> u128 {
    if value >= 0 {
        value as u128
    } else if value == i128::MIN {
        I128_MIN_ABS
    } else {
        (-value) as u128
    }
}

pub(crate) fn signed_from_abs(abs: u128, negative: bool) -> Option<i128> {
    if negative {
        if abs == I128_MIN_ABS {
            Some(i128::MIN)
        } else {
            i128::try_from(abs).ok().and_then(|v| v.checked_neg())
        }
    } else {
        i128::try_from(abs).ok()
    }
}

pub(crate) fn pow10_i128(exp: u32) -> Option<i128> {
    let mut value = 1i128;
    for _ in 0..exp {
        value = value.checked_mul(10)?;
    }
    Some(value)
}

pub(crate) fn pow10_u128(exp: u32) -> Option<u128> {
    let mut value = 1u128;
    for _ in 0..exp {
        value = value.checked_mul(10)?;
    }
    Some(value)
}
