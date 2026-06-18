use core::fmt;

use crate::util::{abs_i128_to_u128, SCALE_FACTOR_U128};
use crate::{Decimal, SCALE};

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rep = format_decimal_abs(self.raw, f.precision());
        f.pad_integral(self.raw >= 0, "", &rep)
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

fn format_decimal_abs(raw: i128, precision: Option<usize>) -> String {
    let abs = abs_i128_to_u128(raw);
    let int = abs / SCALE_FACTOR_U128;
    let frac = abs % SCALE_FACTOR_U128;
    let mut out = int.to_string();

    let Some(precision) = precision else {
        if frac == 0 {
            return out;
        }

        let digits = fractional_digits(frac);
        let mut end = digits.len();

        while end > 0 && digits[end - 1] == b'0' {
            end -= 1;
        }

        out.push('.');
        push_ascii_digits(&mut out, &digits[..end]);
        return out;
    };

    if precision == 0 {
        return out;
    }

    let digits = fractional_digits(frac);
    let fixed_digits = precision.min(SCALE as usize);

    out.push('.');
    push_ascii_digits(&mut out, &digits[..fixed_digits]);
    out.extend(core::iter::repeat_n('0', precision - fixed_digits));
    out
}

fn fractional_digits(mut frac: u128) -> [u8; SCALE as usize] {
    let mut digits = [0u8; SCALE as usize];

    for idx in (0..SCALE as usize).rev() {
        digits[idx] = b'0' + (frac % 10) as u8;
        frac /= 10;
    }

    digits
}

fn push_ascii_digits(out: &mut String, digits: &[u8]) {
    for digit in digits {
        out.push(char::from(*digit));
    }
}
