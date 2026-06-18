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

mod convert;
mod decimal;
mod error;
mod format;
mod ops;
mod parse;
mod rounding;
#[cfg(feature = "serde")]
mod serde;
mod util;
mod wide;

#[cfg(feature = "macros")]
pub use fast_decimal_macros::dec;

pub use convert::{prelude, ToPrimitive};
pub use decimal::Decimal;
pub use error::DecimalError;
pub use rounding::RoundingStrategy;

pub const SCALE: u32 = 12;
pub const SCALE_FACTOR: i128 = 1_000_000_000_000;
