#![cfg(feature = "nohash-hasher")]

use fast_decimal::Decimal;
use nohash_hasher::{IntMap, IntSet};

#[test]
fn decimal_is_an_int_map_key() {
    let positive = Decimal::from_raw(42);
    let negative = Decimal::from_raw(-42);
    let mut map = IntMap::default();

    map.insert(positive, "positive");
    map.insert(negative, "negative");

    assert_eq!(map.get(&positive), Some(&"positive"));
    assert_eq!(map.get(&negative), Some(&"negative"));
}

#[test]
fn decimal_is_an_int_set_key() {
    let values = [Decimal::ZERO, Decimal::ONE, Decimal::MIN, Decimal::MAX];
    let mut set = IntSet::default();

    set.extend(values);

    for value in values {
        assert!(set.contains(&value));
    }
}

#[test]
fn equal_low_bits_are_resolved_by_decimal_equality() {
    let low_bits = Decimal::from_raw(7);
    let same_low_bits = Decimal::from_raw((1_i128 << 64) | 7);
    let mut map = IntMap::default();
    let mut set = IntSet::default();

    map.insert(low_bits, "low");
    map.insert(same_low_bits, "high-and-low");
    set.insert(low_bits);
    set.insert(same_low_bits);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&low_bits), Some(&"low"));
    assert_eq!(map.get(&same_low_bits), Some(&"high-and-low"));
    assert_eq!(set.len(), 2);
    assert!(set.contains(&low_bits));
    assert!(set.contains(&same_low_bits));
}
