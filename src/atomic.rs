use crate::Decimal;
use core::{fmt, sync::atomic::Ordering};
use portable_atomic::AtomicI128;

/// An atomic fixed-scale decimal, available with the `atomic` feature.
///
/// Preserves the full range and precision of [`Decimal`]. Operations follow
/// integer atomic conventions: fetch operations return the previous value,
/// and `fetch_add`/`fetch_sub` wrap on overflow (unlike Decimal operators).
/// Use the checked variants when overflow must leave the value unchanged.
///
/// Memory orderings and their invalid-ordering panics follow
/// [`AtomicI128`]. Atomicity covers this value only, not multi-value transactions.
/// Some targets use a lock fallback; see [`Self::is_lock_free`].
///
/// ```
/// use fast_decimal::{AtomicDecimal, Decimal};
/// use std::sync::atomic::Ordering;
/// let balance = AtomicDecimal::new(Decimal::from(10));
/// assert_eq!(balance.fetch_sub(Decimal::ONE, Ordering::SeqCst), Decimal::from(10));
/// assert_eq!(balance.load(Ordering::SeqCst), Decimal::from(9));
/// ```
pub struct AtomicDecimal {
    raw: AtomicI128,
}

impl AtomicDecimal {
    /// Creates an atomic decimal.
    pub const fn new(value: Decimal) -> Self {
        Self {
            raw: AtomicI128::new(value.raw()),
        }
    }

    /// Returns whether operations on this platform are lock-free.
    pub fn is_lock_free() -> bool {
        AtomicI128::is_lock_free()
    }

    /// Returns whether operations are guaranteed lock-free on all supported CPUs
    /// for the compilation target, without runtime feature detection.
    pub const fn is_always_lock_free() -> bool {
        AtomicI128::is_always_lock_free()
    }

    /// Loads the value. Accepts Relaxed, Acquire or SeqCst; other orderings panic.
    pub fn load(&self, order: Ordering) -> Decimal {
        Decimal::from_raw(self.raw.load(order))
    }

    /// Stores the value. Accepts Relaxed, Release or SeqCst; other orderings panic.
    pub fn store(&self, value: Decimal, order: Ordering) {
        self.raw.store(value.raw(), order);
    }

    /// Replaces the value and returns the previous value. Accepts all orderings.
    pub fn swap(&self, value: Decimal, order: Ordering) -> Decimal {
        Decimal::from_raw(self.raw.swap(value.raw(), order))
    }

    /// Provides exclusive, non-atomic access to the value.
    pub fn get_mut(&mut self) -> &mut Decimal {
        let raw = self.raw.get_mut();
        // SAFETY: Decimal is repr(transparent) over i128, with identical size
        // and alignment, and every i128 bit pattern is valid. The exclusive
        // borrow from get_mut prevents concurrent access and bounds the lifetime.
        unsafe { &mut *(raw as *mut i128).cast::<Decimal>() }
    }

    /// Consumes the atomic and returns its value.
    pub fn into_inner(self) -> Decimal {
        Decimal::from_raw(self.raw.into_inner())
    }

    /// Stores `new` if the value equals `current`, returning the observed old value.
    /// Failure ordering must be Relaxed, Acquire or SeqCst; otherwise panics.
    pub fn compare_exchange(
        &self,
        current: Decimal,
        new: Decimal,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Decimal, Decimal> {
        self.raw
            .compare_exchange(current.raw(), new.raw(), success, failure)
            .map(Decimal::from_raw)
            .map_err(Decimal::from_raw)
    }

    /// Like compare_exchange, but may fail spuriously and is intended for loops.
    /// Failure ordering must be Relaxed, Acquire or SeqCst; otherwise panics.
    pub fn compare_exchange_weak(
        &self,
        current: Decimal,
        new: Decimal,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Decimal, Decimal> {
        self.raw
            .compare_exchange_weak(current.raw(), new.raw(), success, failure)
            .map(Decimal::from_raw)
            .map_err(Decimal::from_raw)
    }

    /// Updates using a CAS loop. Returns the previous value on success, or the
    /// last observed value when the closure returns None. The closure can run
    /// multiple times: it must not rely on exactly-once side effects.
    /// `fetch_order` must be Relaxed, Acquire or SeqCst; otherwise panics.
    pub fn fetch_update<F>(
        &self,
        set_order: Ordering,
        fetch_order: Ordering,
        mut f: F,
    ) -> Result<Decimal, Decimal>
    where
        F: FnMut(Decimal) -> Option<Decimal>,
    {
        self.raw
            .fetch_update(set_order, fetch_order, |raw| {
                f(Decimal::from_raw(raw)).map(Decimal::raw)
            })
            .map(Decimal::from_raw)
            .map_err(Decimal::from_raw)
    }

    /// Adds with raw i128 wrapping semantics, returning the previous value.
    pub fn fetch_add(&self, value: Decimal, order: Ordering) -> Decimal {
        Decimal::from_raw(self.raw.fetch_add(value.raw(), order))
    }

    /// Subtracts with raw i128 wrapping semantics, returning the previous value.
    pub fn fetch_sub(&self, value: Decimal, order: Ordering) -> Decimal {
        Decimal::from_raw(self.raw.fetch_sub(value.raw(), order))
    }

    /// Stores the smaller value and returns the previous value.
    pub fn fetch_min(&self, value: Decimal, order: Ordering) -> Decimal {
        Decimal::from_raw(self.raw.fetch_min(value.raw(), order))
    }

    /// Stores the larger value and returns the previous value.
    pub fn fetch_max(&self, value: Decimal, order: Ordering) -> Decimal {
        Decimal::from_raw(self.raw.fetch_max(value.raw(), order))
    }

    /// Adds without wrapping. Returns the previous value on success, or the
    /// observed value on overflow without writing. Other threads may still write.
    pub fn checked_fetch_add(&self, value: Decimal, order: Ordering) -> Result<Decimal, Decimal> {
        self.fetch_update(order, failure_order(order), |old| old.checked_add(value))
    }

    /// Subtracts without wrapping. Returns the previous value on success, or the
    /// observed value on overflow without writing. This permits negative values.
    pub fn checked_fetch_sub(&self, value: Decimal, order: Ordering) -> Result<Decimal, Decimal> {
        self.fetch_update(order, failure_order(order), |old| old.checked_sub(value))
    }
}

fn failure_order(order: Ordering) -> Ordering {
    match order {
        Ordering::Release => Ordering::Relaxed,
        Ordering::AcqRel => Ordering::Acquire,
        other => other,
    }
}

impl Default for AtomicDecimal {
    fn default() -> Self {
        Self::new(Decimal::ZERO)
    }
}

impl From<Decimal> for AtomicDecimal {
    fn from(value: Decimal) -> Self {
        Self::new(value)
    }
}

impl fmt::Debug for AtomicDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.load(Ordering::Relaxed).fmt(f)
    }
}
