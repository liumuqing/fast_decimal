use crypto_bigint::{NonZero, I128, I256};

use crate::SCALE_FACTOR;

pub(crate) fn checked_mul_wide(lhs: i128, rhs: i128) -> Option<i128> {
    let divisor = NonZero::new(I256::from(SCALE_FACTOR)).into_option()?;
    let quotient = ((I256::from(lhs) * I256::from(rhs)) / divisor).into_option()?;
    i256_to_i128(quotient)
}

pub(crate) fn checked_div_wide(lhs: i128, rhs: i128) -> Option<i128> {
    debug_assert!(rhs != 0);

    let divisor = NonZero::new(I256::from(rhs)).into_option()?;
    let quotient = ((I256::from(lhs) * I256::from(SCALE_FACTOR)) / divisor).into_option()?;
    i256_to_i128(quotient)
}

fn i256_to_i128(value: I256) -> Option<i128> {
    if value < I256::from(i128::MIN) || value > I256::from(i128::MAX) {
        return None;
    }

    Some(i128::from(I128::from(&value)))
}
