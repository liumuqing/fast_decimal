use core::str::FromStr;

use crate::{Decimal, DecimalError, SCALE_FACTOR};

macro_rules! impl_from_integer {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Decimal {
                fn from(value: $ty) -> Self {
                    Decimal::from_integer(value as i128)
                }
            }
        )*
    };
}

impl_from_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, usize);

impl From<u128> for Decimal {
    fn from(value: u128) -> Self {
        let value = i128::try_from(value).expect("Decimal u128 conversion overflow");
        Decimal::from_integer(value)
    }
}

impl TryFrom<f64> for Decimal {
    type Error = DecimalError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(DecimalError::NonFiniteFloat);
        }

        let s = format!("{value:.18}");
        Decimal::from_str(&s)
    }
}

impl TryFrom<Decimal> for f64 {
    type Error = DecimalError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        Ok(value.raw as f64 / SCALE_FACTOR as f64)
    }
}

impl TryFrom<Decimal> for u128 {
    type Error = DecimalError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        if value.raw < 0 {
            return Err(DecimalError::NegativeToUnsigned);
        }
        if value.raw % SCALE_FACTOR != 0 {
            return Err(DecimalError::FractionalToInteger);
        }
        Ok((value.raw / SCALE_FACTOR) as u128)
    }
}

pub mod prelude {
    pub use crate::ToPrimitive;
}

pub trait ToPrimitive {
    fn to_u128(&self) -> Option<u128>;
}

impl ToPrimitive for Decimal {
    fn to_u128(&self) -> Option<u128> {
        Decimal::to_u128(self)
    }
}
