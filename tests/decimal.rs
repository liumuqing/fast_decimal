use core::str::FromStr;

use fast_decimal::{Decimal, DecimalError, RoundingStrategy, SCALE_FACTOR};

#[test]
fn parses_and_displays_fixed_scale_values() {
    assert_eq!(Decimal::from_str("0").unwrap().raw(), 0);
    assert_eq!(Decimal::from_str("1").unwrap().raw(), SCALE_FACTOR);
    assert_eq!(
        Decimal::from_str("-1.25").unwrap().raw(),
        -1_250_000_000_000
    );
    assert_eq!(Decimal::from_str(".5").unwrap().to_string(), "0.5");
    assert_eq!(Decimal::from_str("1.230000").unwrap().to_string(), "1.23");
}

#[test]
fn from_str_rounds_extra_fractional_digits() {
    assert_eq!(
        Decimal::from_str("1.1234567890123").unwrap().to_string(),
        "1.123456789012"
    );
    assert_eq!(
        Decimal::from_str("1.1234567890125").unwrap().to_string(),
        "1.123456789013"
    );
    assert_eq!(
        Decimal::from_str("-1.1234567890125").unwrap().to_string(),
        "-1.123456789013"
    );
}

#[test]
fn rejects_invalid_parse_inputs() {
    assert_eq!(
        Decimal::from_str("").unwrap_err(),
        DecimalError::InvalidSyntax
    );
    assert_eq!(
        Decimal::from_str("+").unwrap_err(),
        DecimalError::InvalidSyntax
    );
    assert_eq!(
        Decimal::from_str(".").unwrap_err(),
        DecimalError::InvalidSyntax
    );
    assert_eq!(
        Decimal::from_str("1.2.3").unwrap_err(),
        DecimalError::InvalidSyntax
    );
    assert_eq!(
        Decimal::from_str("1e3").unwrap_err(),
        DecimalError::InvalidSyntax
    );
}

#[test]
fn constructors_match_rust_decimal_shape() {
    assert_eq!(Decimal::new(1, 3).to_string(), "0.001");
    assert_eq!(
        Decimal::from_i128_with_scale(15, 13).to_string(),
        "0.000000000002"
    );
    assert_eq!(Decimal::from(42u64).to_string(), "42");
}

#[test]
fn arithmetic_uses_decimal_scale() {
    let a = Decimal::from_str("1.5").unwrap();
    let b = Decimal::from_str("2").unwrap();
    let c = Decimal::from_str("0.25").unwrap();

    assert_eq!((a + b).to_string(), "3.5");
    assert_eq!((a - c).to_string(), "1.25");
    assert_eq!((a * b).to_string(), "3");
    assert_eq!((a / b).to_string(), "0.75");
}

#[test]
fn multiplication_uses_wide_intermediate_when_needed() {
    let lhs = Decimal::from_raw(10_i128.pow(24));
    let rhs = Decimal::from_raw(10_i128.pow(24));

    let product = lhs.checked_mul(rhs).unwrap();

    assert_eq!(product.raw(), 10_i128.pow(36));
    assert_eq!(product.to_string(), "1000000000000000000000000");
}

#[test]
fn division_uses_wide_intermediate_when_needed() {
    let lhs = Decimal::from_raw(10_i128.pow(36));
    let rhs = Decimal::from_raw(10_i128.pow(24));

    let quotient = lhs.checked_div(rhs).unwrap();

    assert_eq!(quotient.raw(), 10_i128.pow(24));
    assert_eq!(quotient.to_string(), "1000000000000");
}

#[test]
fn wide_arithmetic_preserves_sign_and_overflow_checks() {
    let lhs = Decimal::from_raw(-10_i128.pow(24));
    let rhs = Decimal::from_raw(10_i128.pow(24));

    assert_eq!(lhs.checked_mul(rhs).unwrap().raw(), -10_i128.pow(36));
    assert_eq!(
        Decimal::from_raw(10_i128.pow(36))
            .checked_div(Decimal::from_raw(-10_i128.pow(24)))
            .unwrap()
            .raw(),
        -10_i128.pow(24)
    );

    assert_eq!(Decimal::MAX.checked_mul(Decimal::MAX), None);
    assert_eq!(Decimal::MAX.checked_div(Decimal::from_raw(1)), None);
}

#[test]
fn rounding_strategies_cover_polymarket_usage() {
    let value = Decimal::from_str("1.23456").unwrap();
    let neg = Decimal::from_str("-1.23456").unwrap();

    assert_eq!(
        value
            .round_dp_with_strategy(3, RoundingStrategy::ToZero)
            .to_string(),
        "1.234"
    );
    assert_eq!(
        value
            .round_dp_with_strategy(3, RoundingStrategy::AwayFromZero)
            .to_string(),
        "1.235"
    );
    assert_eq!(value.round_dp(4).to_string(), "1.2346");
    assert_eq!(
        neg.round_dp_with_strategy(3, RoundingStrategy::AwayFromZero)
            .to_string(),
        "-1.235"
    );

    let midpoint = Decimal::from_str("-1.2345").unwrap();
    assert_eq!(
        midpoint
            .round_dp_with_strategy(3, RoundingStrategy::MidpointTowardZero)
            .to_string(),
        "-1.234"
    );
    assert_eq!(
        midpoint
            .round_dp_with_strategy(3, RoundingStrategy::MidpointAwayFromZero)
            .to_string(),
        "-1.235"
    );
}

#[test]
fn floor_trunc_and_integer_conversion() {
    let value = Decimal::from_str("-1.25").unwrap();
    assert_eq!(value.floor().to_string(), "-2");
    assert_eq!(value.trunc().to_string(), "-1");
    assert_eq!(value.trunc_with_scale(1).to_string(), "-1.2");

    let whole = Decimal::from_str("123").unwrap();
    let converted: u128 = whole.try_into().unwrap();
    assert_eq!(converted, 123);
    assert!(u128::try_from(Decimal::from_str("123.1").unwrap()).is_err());
}

#[test]
fn checked_arithmetic_reports_failures() {
    assert_eq!(Decimal::MAX.checked_add(Decimal::ONE), None);
    assert_eq!(Decimal::MIN.checked_sub(Decimal::ONE), None);
    assert_eq!(Decimal::MIN.checked_neg(), None);
    assert_eq!(Decimal::ONE.checked_div(Decimal::ZERO), None);
}

#[test]
fn operators_panic_on_overflow_or_division_by_zero() {
    assert!(std::panic::catch_unwind(|| Decimal::MAX + Decimal::ONE).is_err());
    assert!(std::panic::catch_unwind(|| Decimal::MIN - Decimal::ONE).is_err());
    assert!(std::panic::catch_unwind(|| -Decimal::MIN).is_err());
    assert!(std::panic::catch_unwind(|| Decimal::ONE / Decimal::ZERO).is_err());
}
