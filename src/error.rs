use core::fmt;

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
