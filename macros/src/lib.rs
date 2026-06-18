use proc_macro::TokenStream;

const SCALE: u32 = 12;
const SCALE_FACTOR: u128 = 1_000_000_000_000;
const I128_MIN_ABS: u128 = 1u128 << 127;

#[proc_macro]
pub fn dec(input: TokenStream) -> TokenStream {
    match parse_decimal_literal(&input.to_string()) {
        Ok(raw) => {
            let decimal_path = if cfg!(feature = "rust_decimal_path") {
                "::rust_decimal::Decimal"
            } else {
                "::fast_decimal::Decimal"
            };

            format!("{decimal_path}::from_raw({raw}i128)")
                .parse()
                .expect("generated decimal tokens must parse")
        }
        Err(message) => format!("compile_error!({message:?})")
            .parse()
            .expect("generated compile_error tokens must parse"),
    }
}

fn parse_decimal_literal(input: &str) -> Result<i128, String> {
    let mut s = input.trim().replace([' ', '_'], "");

    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s = s[1..s.len() - 1].to_owned();
    }

    if s.is_empty() {
        return Err("empty decimal literal".to_owned());
    }

    let mut chars = s.as_bytes();
    let mut negative = false;
    if chars[0] == b'-' {
        negative = true;
        chars = &chars[1..];
    } else if chars[0] == b'+' {
        chars = &chars[1..];
    }

    if chars.is_empty() {
        return Err("invalid decimal literal".to_owned());
    }

    let mut int = 0u128;
    let mut frac = 0u128;
    let mut frac_len = 0u32;
    let mut extra_non_zero_digit = false;
    let mut saw_digit = false;
    let mut saw_dot = false;

    for &b in chars {
        match b {
            b'.' if !saw_dot => saw_dot = true,
            b'0'..=b'9' => {
                saw_digit = true;
                let digit = (b - b'0') as u128;

                if saw_dot {
                    if frac_len < SCALE {
                        frac = frac
                            .checked_mul(10)
                            .and_then(|v| v.checked_add(digit))
                            .ok_or_else(|| "decimal literal overflow".to_owned())?;
                        frac_len += 1;
                    } else if digit != 0 {
                        extra_non_zero_digit = true;
                    }
                } else {
                    int = int
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(digit))
                        .ok_or_else(|| "decimal literal overflow".to_owned())?;
                }
            }
            _ => return Err(format!("invalid decimal literal: {input}")),
        }
    }

    if !saw_digit {
        return Err("invalid decimal literal".to_owned());
    }

    for _ in frac_len..SCALE {
        frac = frac
            .checked_mul(10)
            .ok_or_else(|| "decimal literal overflow".to_owned())?;
    }

    let raw_abs = int
        .checked_mul(SCALE_FACTOR)
        .and_then(|v| v.checked_add(frac))
        .ok_or_else(|| "decimal literal overflow".to_owned())?;

    if extra_non_zero_digit {
        return Err(format!(
            "decimal literal has more than {SCALE} fractional digits"
        ));
    }

    signed_from_abs(raw_abs, negative).ok_or_else(|| "decimal literal overflow".to_owned())
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
