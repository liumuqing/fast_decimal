# fast_decimal

`fast_decimal` is a fast fixed-scale decimal for Rust code.

It stores a signed `i128` raw value with a fixed scale of 12:

```text
value = raw / 1_000_000_000_000
```

The public type is:

```rust
pub struct Decimal {
    raw: i128,
}
```

## Goals

- Fast arithmetic with a fixed decimal scale.
- Small `rust_decimal`-style API surface for easier migration.
- Deterministic decimal behavior with no floating point in core arithmetic.
- Compile-time decimal literals through `fast_decimal_macros::dec!`.

## Non-goals

- Full `rust_decimal` compatibility.
- Dynamic scale semantics.
- Arbitrary precision arithmetic.
- Dynamic scale arithmetic.

## Rounding

Runtime parsing rounds to 12 fractional digits:

```rust
use fast_decimal::Decimal;
use std::str::FromStr;

assert_eq!(
    Decimal::from_str("1.1234567890125").unwrap().to_string(),
    "1.123456789013"
);
```

The `dec!` macro is stricter and does not round non-zero digits past 12 places:

```rust
use fast_decimal::Decimal;
use fast_decimal_macros::dec;

const TICK: Decimal = dec!(0.001);
```

`dec!(1.1234567890120)` is allowed. `dec!(1.1234567890125)` is a compile error.

## Arithmetic

Operators panic on overflow or division by zero:

```rust
let z = x * y;
```

Use checked APIs when failures should be explicit:

```rust
let z = x.checked_mul(y);
```

Multiplication and division currently use `i128` intermediate arithmetic. This is fast, but conservative near the limits of `i128`.

## Serde

Enable the `serde` feature:

```toml
fast_decimal = { version = "0.1", features = ["serde"] }
```

Serialization emits a decimal string. Deserialization accepts decimal strings and JSON numbers.

## Cargo Alias Migration

For code that imports `rust_decimal` and `rust_decimal_macros`, a local migration can use package aliases:

```toml
rust_decimal = { package = "fast_decimal", path = "../fixed-decimal", features = ["serde"] }
rust_decimal_macros = { package = "fast_decimal_macros", path = "../fixed-decimal/macros", features = ["rust_decimal_path"] }
```

The `rust_decimal_path` feature makes `dec!` expand to `::rust_decimal::Decimal::from_raw(...)`.

## License

Licensed under the MIT license. See [LICENSE](LICENSE).
