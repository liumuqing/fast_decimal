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

Multiplication and division use a fast `i128` path first, then fall back to internal wide arithmetic when the intermediate product would overflow.

## Serde

Enable the `serde` feature:

```toml
fast_decimal = { version = "0.1", features = ["serde"] }
```

Serialization emits a decimal string. Deserialization accepts decimal strings and JSON numbers.

## Integer maps and sets

Enable the `nohash-hasher` feature to use `Decimal` directly with
`nohash_hasher::IntMap` and `nohash_hasher::IntSet`:

```toml
fast_decimal = { version = "0.1", features = ["nohash-hasher"] }
```

```rust
use fast_decimal::Decimal;
use nohash_hasher::{IntMap, IntSet};

let mut quantities = IntMap::<Decimal, Decimal>::default();
quantities.insert(Decimal::ONE, Decimal::from_integer(2));

let mut prices = IntSet::<Decimal>::default();
prices.insert(Decimal::ONE);
```

`Decimal` hashes its fixed-scale raw value with one `write_u64` call using the
low 64 bits. Values that differ only in the upper 64 bits collide but remain
distinct because `IntMap` and `IntSet` still compare keys with `Eq`.

## Cargo Alias Migration

For code that imports `rust_decimal` and `rust_decimal_macros`, a local migration can use package aliases:

```toml
rust_decimal = { package = "fast_decimal", path = "../fixed-decimal", features = ["serde"] }
rust_decimal_macros = { package = "fast_decimal_macros", path = "../fixed-decimal/macros", features = ["rust_decimal_path"] }
```

The `rust_decimal_path` feature makes `dec!` expand to `::rust_decimal::Decimal::from_raw(...)`.

## License

Licensed under the MIT license. See [LICENSE](LICENSE).

## Atomic decimals

Enable `atomic` to use `AtomicDecimal`, backed by `portable-atomic::AtomicI128`:

```toml
fast_decimal = { version = "0.1", features = ["atomic"] }
```

```rust
use fast_decimal::{AtomicDecimal, Decimal};
use std::sync::atomic::Ordering;

let balance = AtomicDecimal::new(Decimal::from(10));
let previous = balance.checked_fetch_sub(Decimal::ONE, Ordering::SeqCst).unwrap();
assert_eq!(previous, Decimal::from(10));
assert_eq!(balance.load(Ordering::SeqCst), Decimal::from(9));
```

The interface follows integer atomics: load/store/swap, strong and weak CAS,
fetch_add/sub/min/max, fetch_update, get_mut and into_inner. All values retain
Decimal's full i128 range and 12-place precision. `Default`, `From<Decimal>` and
`Debug` are implemented; Send/Sync are automatic. There is no implicit cloning,
comparison or arithmetic assignment of an atomic value.

Unlike ordinary Decimal arithmetic, `fetch_add/sub` wrap the raw i128 on overflow.
`checked_fetch_add/sub` return `Ok(previous)` or `Err(observed)` on overflow
without writing. They permit negative results. For a nonnegative balance, use
`fetch_update` to combine the balance check and subtraction in one atomic update;
separate load and subtraction operations can race. Its closure may run repeatedly
and must not rely on exactly-once side effects.

Memory ordering and invalid-ordering panics match the underlying integer atomics.
Debug formatting performs a Relaxed load. A single atomic value does not provide
a transaction across several balances or positions. Lock freedom is platform
dependent: `is_lock_free()` reports runtime support and `is_always_lock_free()`
reports the compile-time guarantee. Unsupported targets use the dependency's lock
fallback. No raw-pointer or bitwise decimal operations are exposed.
