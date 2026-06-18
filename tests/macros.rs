use fast_decimal::Decimal;
use fast_decimal_macros::dec;

const TICK: Decimal = dec!(0.001);
const TRAILING_ZERO: Decimal = dec!(1.1234567890120);

#[test]
fn macro_builds_const_decimals_from_literals() {
    assert_eq!(TICK.to_string(), "0.001");
    assert_eq!(TRAILING_ZERO.to_string(), "1.123456789012");
    assert_eq!(dec!(-1.25).to_string(), "-1.25");
    assert_eq!(dec!(1_000_000).to_string(), "1000000");
}
