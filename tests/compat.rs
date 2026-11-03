use core::str::FromStr;

use fast_decimal::prelude::ToPrimitive;
use fast_decimal::Decimal;

#[test]
fn exposes_rust_decimal_style_sign_and_primitive_helpers() {
    let negative = Decimal::from_str("-1.25").unwrap();
    let positive = Decimal::from_str("123").unwrap();
    let fractional = Decimal::from_str("123.1").unwrap();

    assert!(negative.is_sign_negative());
    assert!(!positive.is_sign_negative());
    assert_eq!(positive.to_u128(), Some(123));
    assert_eq!(ToPrimitive::to_u128(&positive), Some(123));
    assert_eq!(fractional.to_u128(), None);
}

#[cfg(feature = "serde")]
#[test]
fn serde_uses_decimal_strings_and_accepts_numbers() {
    let value = Decimal::from_str("1.25").unwrap();
    assert_eq!(serde_json::to_string(&value).unwrap(), "\"1.25\"");

    assert_eq!(serde_json::from_str::<Decimal>("\"1.250\"").unwrap(), value);
    assert_eq!(serde_json::from_str::<Decimal>("1.25").unwrap(), value);
    assert_eq!(
        serde_json::from_str::<Decimal>("2").unwrap(),
        Decimal::from(2u64)
    );
}

#[cfg(feature = "serde")]
#[test]
fn json_numbers_keep_decimal_precision_without_extra_features() {
    for raw in [
        "1234567890.123456789",
        "1.234567890123456789e9",
        "-5E-7",
        "5e-13",
        "1e-1000",
    ] {
        let expected = raw.parse::<Decimal>().unwrap();
        assert_eq!(serde_json::from_str::<Decimal>(raw).unwrap(), expected);
        let value: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(serde_json::from_value::<Decimal>(value).unwrap(), expected);
        assert_eq!(
            serde_json::from_str::<Option<Decimal>>(raw).unwrap(),
            Some(expected)
        );
        let quoted = serde_json::to_string(raw).unwrap();
        assert_eq!(serde_json::from_str::<Decimal>(&quoted).unwrap(), expected);
    }
    assert_eq!(
        serde_json::from_str::<Option<Decimal>>("null").unwrap(),
        None
    );
    // Preserve the library's existing serialized representation.
    let value = "1234567890.123456789".parse::<Decimal>().unwrap();
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        "\"1234567890.123456789\""
    );
}

#[cfg(feature = "serde")]
#[test]
fn json_decimal_rejects_non_numbers_and_overflow() {
    for raw in [
        "null",
        "true",
        "[]",
        "{}",
        "{\"amount\":1}",
        "1e1000",
        "\"1e+\"",
        "170141183460469231731687303715884105728e-12",
    ] {
        assert!(serde_json::from_str::<Decimal>(raw).is_err(), "{raw}");
    }
    for (raw, expected) in [
        ("170141183460469231731687303715884105727e-12", i128::MAX),
        ("-170141183460469231731687303715884105728e-12", i128::MIN),
    ] {
        assert_eq!(
            serde_json::from_str::<Decimal>(raw).unwrap().raw(),
            expected
        );
    }
}
