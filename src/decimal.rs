use crate::rounding::{round_to_unit, scale_mantissa};
use crate::wide::{checked_div_wide, checked_mul_wide};
use crate::{DecimalError, RoundingStrategy, SCALE, SCALE_FACTOR};

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Decimal {
    pub(crate) raw: i128,
}

impl Decimal {
    pub const ZERO: Self = Self { raw: 0 };
    pub const ONE: Self = Self { raw: SCALE_FACTOR };
    pub const MAX: Self = Self { raw: i128::MAX };
    pub const MIN: Self = Self { raw: i128::MIN };

    pub const fn from_raw(raw: i128) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> i128 {
        self.raw
    }

    pub fn new(mantissa: i64, scale: u32) -> Self {
        Self::from_i128_with_scale(mantissa as i128, scale)
    }

    pub fn from_i128_with_scale(mantissa: i128, scale: u32) -> Self {
        Self::checked_from_i128_with_scale(mantissa, scale)
            .expect("Decimal::from_i128_with_scale overflow")
    }

    pub fn checked_from_i128_with_scale(mantissa: i128, scale: u32) -> Option<Self> {
        scale_mantissa(mantissa, scale, RoundingStrategy::MidpointAwayFromZero)
    }

    pub fn from_str_exact(s: &str) -> Result<Self, DecimalError> {
        s.parse()
    }

    pub fn from_f64_retain(value: f64) -> Option<Self> {
        Self::try_from(value).ok()
    }

    pub const fn scale(&self) -> u32 {
        SCALE
    }

    pub const fn is_zero(&self) -> bool {
        self.raw == 0
    }

    pub const fn is_sign_negative(&self) -> bool {
        self.raw < 0
    }

    pub const fn mantissa(&self) -> i128 {
        self.raw
    }

    pub fn to_u128(&self) -> Option<u128> {
        (*self).try_into().ok()
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.raw.checked_add(rhs.raw).map(Self::from_raw)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.raw.checked_sub(rhs.raw).map(Self::from_raw)
    }

    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        if let Some(raw) = self
            .raw
            .checked_mul(rhs.raw)
            .and_then(|raw| raw.checked_div(SCALE_FACTOR))
        {
            return Some(Self::from_raw(raw));
        }

        checked_mul_wide(self.raw, rhs.raw).map(Self::from_raw)
    }

    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.raw == 0 {
            return None;
        }

        if let Some(raw) = self
            .raw
            .checked_mul(SCALE_FACTOR)
            .and_then(|raw| raw.checked_div(rhs.raw))
        {
            return Some(Self::from_raw(raw));
        }

        checked_div_wide(self.raw, rhs.raw).map(Self::from_raw)
    }

    pub fn checked_neg(self) -> Option<Self> {
        self.raw.checked_neg().map(Self::from_raw)
    }

    pub fn abs(self) -> Self {
        if self.raw == i128::MIN {
            panic!("Decimal absolute value overflow");
        }
        Self::from_raw(self.raw.abs())
    }

    pub fn floor(self) -> Self {
        let q = self.raw / SCALE_FACTOR;
        let r = self.raw % SCALE_FACTOR;
        if self.raw < 0 && r != 0 {
            Self::from_integer(q - 1)
        } else {
            Self::from_integer(q)
        }
    }

    pub fn trunc(self) -> Self {
        Self::from_raw((self.raw / SCALE_FACTOR) * SCALE_FACTOR)
    }

    pub fn round(self) -> Self {
        self.round_dp(0)
    }

    pub fn round_dp(self, dp: u32) -> Self {
        self.round_dp_with_strategy(dp, RoundingStrategy::MidpointAwayFromZero)
    }

    pub fn round_dp_with_strategy(self, dp: u32, strategy: RoundingStrategy) -> Self {
        if dp >= SCALE {
            return self;
        }

        let unit = crate::util::pow10_i128(SCALE - dp).expect("rounding scale overflow");
        round_to_unit(self.raw, unit, strategy)
    }

    pub fn trunc_with_scale(self, scale: u32) -> Self {
        self.round_dp_with_strategy(scale, RoundingStrategy::ToZero)
    }

    pub fn from_integer(value: i128) -> Self {
        Self::checked_from_integer(value).expect("Decimal integer conversion overflow")
    }

    pub fn checked_from_integer(value: i128) -> Option<Self> {
        value.checked_mul(SCALE_FACTOR).map(Self::from_raw)
    }
}
