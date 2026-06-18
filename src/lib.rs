//! A fast fixed-scale decimal for Rust code.
//!
//! `Decimal` stores a signed `i128` raw value with a fixed scale of 12:
//!
//! ```text
//! numeric value = raw / 1_000_000_000_000
//! ```
//!
//! This crate intentionally does not implement `rust_decimal`'s dynamic-scale
//! semantics. It provides a small compatibility-oriented API surface for code
//! that mostly needs fast arithmetic, parsing, formatting, rounding, serde, and
//! `dec!` literals.
//!
//! Important behavior:
//!
//! - `FromStr` rounds extra fractional digits to 12 decimal places using
//!   [`RoundingStrategy::MidpointAwayFromZero`].
//! - `fast_decimal_macros::dec!` is stricter: non-zero digits past 12 fractional
//!   places are a compile error.
//! - ordinary arithmetic operators panic on overflow or division by zero,
//!   matching the ergonomic model of `rust_decimal`.
//! - `checked_*` methods return `None` instead.
//! - multiplication and division use internal wide arithmetic when the fast
//!   `i128` intermediate path would overflow.
//!
//! # Examples
//!
//! ```
//! use fast_decimal::Decimal;
//! use std::str::FromStr;
//!
//! let amount = Decimal::from_str("10.5").unwrap();
//! let price = Decimal::from_str("0.42").unwrap();
//! assert_eq!((amount * price).to_string(), "4.41");
//! ```
//!
//! With the `macros` feature:
//!
//! ```ignore
//! use fast_decimal::{dec, Decimal};
//!
//! const TICK: Decimal = dec!(0.001);
//! ```

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use core::str::FromStr;

#[cfg(feature = "macros")]
pub use fast_decimal_macros::dec;

pub const SCALE: u32 = 12;
pub const SCALE_FACTOR: i128 = 1_000_000_000_000;

const SCALE_FACTOR_U128: u128 = SCALE_FACTOR as u128;
const I128_MIN_ABS: u128 = 1u128 << 127;

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Decimal {
    raw: i128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoundingStrategy {
    ToZero,
    AwayFromZero,
    MidpointTowardZero,
    MidpointAwayFromZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecimalError {
    InvalidSyntax,
    Overflow,
    DivisionByZero,
    NegativeToUnsigned,
    FractionalToInteger,
    NonFiniteFloat,
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntax => f.write_str("invalid decimal syntax"),
            Self::Overflow => f.write_str("decimal overflow"),
            Self::DivisionByZero => f.write_str("division by zero"),
            Self::NegativeToUnsigned => {
                f.write_str("negative decimal cannot convert to unsigned integer")
            }
            Self::FractionalToInteger => f.write_str("decimal has a fractional part"),
            Self::NonFiniteFloat => f.write_str("float is NaN or infinite"),
        }
    }
}

impl std::error::Error for DecimalError {}

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
        Self::from_str(s)
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

        let unit = pow10_i128(SCALE - dp).expect("rounding scale overflow");
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

impl FromStr for Decimal {
    type Err = DecimalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_decimal(s)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.raw == 0 {
            return f.write_str("0");
        }

        let negative = self.raw < 0;
        let abs = abs_i128_to_u128(self.raw);
        let int = abs / SCALE_FACTOR_U128;
        let mut frac = abs % SCALE_FACTOR_U128;

        if negative {
            f.write_str("-")?;
        }

        if frac == 0 {
            return write!(f, "{int}");
        }

        let mut digits = [0u8; SCALE as usize];
        for idx in (0..SCALE as usize).rev() {
            digits[idx] = b'0' + (frac % 10) as u8;
            frac /= 10;
        }

        let mut end = digits.len();
        while end > 0 && digits[end - 1] == b'0' {
            end -= 1;
        }

        write!(f, "{int}.")?;
        for digit in &digits[..end] {
            f.write_str(core::str::from_utf8(&[*digit]).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Add for Decimal {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("Decimal addition overflow")
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("Decimal subtraction overflow")
    }
}

impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs)
            .expect("Decimal multiplication overflow")
    }
}

impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).expect("Decimal division failed")
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.checked_neg().expect("Decimal negation overflow")
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Decimal {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign for Decimal {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl DivAssign for Decimal {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

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

#[cfg(feature = "serde")]
impl serde::Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DecimalVisitor;

        impl<'de> serde::de::Visitor<'de> for DecimalVisitor {
            type Value = Decimal;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a decimal string or number")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Decimal::from_str(value).map_err(E::custom)
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(value)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Decimal::from(value))
            }

            fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Decimal::from(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Decimal::from(value))
            }

            fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i128::try_from(value)
                    .map(Decimal::from)
                    .map_err(|_| E::custom(DecimalError::Overflow))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Decimal::try_from(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(DecimalVisitor)
    }
}

fn parse_decimal(s: &str) -> Result<Decimal, DecimalError> {
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

fn checked_mul_wide(lhs: i128, rhs: i128) -> Option<i128> {
    let negative = (lhs < 0) ^ (rhs < 0);
    let lhs_abs = abs_i128_to_u128(lhs);
    let rhs_abs = abs_i128_to_u128(rhs);
    let product = U256::mul_u128(lhs_abs, rhs_abs);
    let quotient = product.div_u128_to_u128(SCALE_FACTOR_U128)?;

    signed_from_abs(quotient, negative)
}

fn checked_div_wide(lhs: i128, rhs: i128) -> Option<i128> {
    debug_assert!(rhs != 0);

    let negative = (lhs < 0) ^ (rhs < 0);
    let lhs_abs = abs_i128_to_u128(lhs);
    let rhs_abs = abs_i128_to_u128(rhs);
    let numerator = U256::mul_u128(lhs_abs, SCALE_FACTOR_U128);
    let quotient = numerator.div_u128_to_u128(rhs_abs)?;

    signed_from_abs(quotient, negative)
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct U256 {
    hi: u128,
    lo: u128,
}

impl U256 {
    const ZERO: Self = Self { hi: 0, lo: 0 };

    const fn from_u128(value: u128) -> Self {
        Self { hi: 0, lo: value }
    }

    fn mul_u128(lhs: u128, rhs: u128) -> Self {
        const MASK: u128 = u64::MAX as u128;

        let lhs_lo = lhs & MASK;
        let lhs_hi = lhs >> 64;
        let rhs_lo = rhs & MASK;
        let rhs_hi = rhs >> 64;

        let p0 = lhs_lo * rhs_lo;
        let p1 = lhs_lo * rhs_hi;
        let p2 = lhs_hi * rhs_lo;
        let p3 = lhs_hi * rhs_hi;

        let lo_low = p0 & MASK;
        let mid = (p0 >> 64) + (p1 & MASK) + (p2 & MASK);
        let lo_high = mid & MASK;
        let hi = p3 + (p1 >> 64) + (p2 >> 64) + (mid >> 64);

        Self {
            hi,
            lo: lo_low | (lo_high << 64),
        }
    }

    fn div_u128_to_u128(self, divisor: u128) -> Option<u128> {
        debug_assert!(divisor != 0);
        let quotient = self.div_u256(Self::from_u128(divisor));
        quotient.to_u128()
    }

    fn div_u256(self, divisor: Self) -> Self {
        debug_assert!(divisor != Self::ZERO);

        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;

        for bit in (0..256).rev() {
            remainder.shl1_add_bit(self.bit(bit));

            if remainder >= divisor {
                remainder.sub_assign(divisor);
                quotient.set_bit(bit);
            }
        }

        quotient
    }

    fn to_u128(self) -> Option<u128> {
        if self.hi == 0 {
            Some(self.lo)
        } else {
            None
        }
    }

    fn bit(self, bit: u32) -> bool {
        debug_assert!(bit < 256);

        if bit < 128 {
            ((self.lo >> bit) & 1) != 0
        } else {
            ((self.hi >> (bit - 128)) & 1) != 0
        }
    }

    fn set_bit(&mut self, bit: u32) {
        debug_assert!(bit < 256);

        if bit < 128 {
            self.lo |= 1u128 << bit;
        } else {
            self.hi |= 1u128 << (bit - 128);
        }
    }

    fn shl1_add_bit(&mut self, bit: bool) {
        self.hi = (self.hi << 1) | (self.lo >> 127);
        self.lo = (self.lo << 1) | u128::from(bit);
    }

    fn sub_assign(&mut self, rhs: Self) {
        let (lo, borrow) = self.lo.overflowing_sub(rhs.lo);
        self.lo = lo;
        self.hi = self.hi - rhs.hi - u128::from(borrow);
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hi.cmp(&other.hi) {
            Ordering::Equal => self.lo.cmp(&other.lo),
            ordering => ordering,
        }
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn scale_mantissa(mantissa: i128, scale: u32, strategy: RoundingStrategy) -> Option<Decimal> {
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

fn round_to_unit(raw: i128, unit: i128, strategy: RoundingStrategy) -> Decimal {
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

fn abs_i128_to_u128(value: i128) -> u128 {
    if value >= 0 {
        value as u128
    } else if value == i128::MIN {
        I128_MIN_ABS
    } else {
        (-value) as u128
    }
}

fn signed_from_abs(abs: u128, negative: bool) -> Option<i128> {
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

fn pow10_i128(exp: u32) -> Option<i128> {
    let mut value = 1i128;
    for _ in 0..exp {
        value = value.checked_mul(10)?;
    }
    Some(value)
}

fn pow10_u128(exp: u32) -> Option<u128> {
    let mut value = 1u128;
    for _ in 0..exp {
        value = value.checked_mul(10)?;
    }
    Some(value)
}
