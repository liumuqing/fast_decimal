use fast_decimal::Decimal;
use fast_decimal_macros::dec;

const BAD: Decimal = dec!(1.1234567890121);

fn main() {
    let _ = BAD;
}
