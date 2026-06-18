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
