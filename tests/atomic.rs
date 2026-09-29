#![cfg(feature = "atomic")]
use fast_decimal::{AtomicDecimal, Decimal};
use std::sync::{atomic::Ordering::*, Barrier};

#[test]
fn access_traits_and_extrema() {
    fn assert_traits<T: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe>() {}
    assert_traits::<AtomicDecimal>();
    let mut value = const { AtomicDecimal::new(Decimal::ONE) };
    assert_eq!(value.load(Relaxed), Decimal::ONE);
    value.store(Decimal::MIN, Release);
    assert_eq!(value.swap(Decimal::MAX, AcqRel), Decimal::MIN);
    assert_eq!(value.fetch_min(Decimal::ZERO, SeqCst), Decimal::MAX);
    assert_eq!(value.fetch_max(Decimal::ONE, Relaxed), Decimal::ZERO);
    *value.get_mut() = Decimal::from_raw(-1);
    assert_eq!(format!("{value:?}"), format!("{:?}", Decimal::from_raw(-1)));
    assert_eq!(value.into_inner(), Decimal::from_raw(-1));
    assert_eq!(AtomicDecimal::default().into_inner(), Decimal::ZERO);
    assert_eq!(AtomicDecimal::from(Decimal::MAX).into_inner(), Decimal::MAX);
    assert!(!AtomicDecimal::is_always_lock_free() || AtomicDecimal::is_lock_free());
}

#[test]
fn cas_and_conditional_updates() {
    let value = AtomicDecimal::new(Decimal::ONE);
    assert_eq!(
        value.compare_exchange(Decimal::ZERO, Decimal::MIN, AcqRel, Acquire),
        Err(Decimal::ONE)
    );
    assert_eq!(
        value.compare_exchange(Decimal::ONE, Decimal::ZERO, Release, Relaxed),
        Ok(Decimal::ONE)
    );
    loop {
        if value
            .compare_exchange_weak(Decimal::ZERO, Decimal::ONE, SeqCst, SeqCst)
            .is_ok()
        {
            break;
        }
    }
    assert_eq!(
        value.fetch_update(SeqCst, SeqCst, |_| None),
        Err(Decimal::ONE)
    );
    assert_eq!(
        value.fetch_update(AcqRel, Acquire, |old| old.checked_add(Decimal::ONE)),
        Ok(Decimal::ONE)
    );
    assert_eq!(value.into_inner(), Decimal::from(2));
}

#[test]
fn wrapping_and_checked_arithmetic_preserve_full_precision() {
    let tick = Decimal::from_raw(1);
    let value = AtomicDecimal::new(Decimal::MAX);
    assert_eq!(value.checked_fetch_add(tick, Release), Err(Decimal::MAX));
    assert_eq!(value.load(Relaxed), Decimal::MAX);
    assert_eq!(value.fetch_add(tick, Relaxed), Decimal::MAX);
    assert_eq!(value.load(Relaxed), Decimal::MIN);
    assert_eq!(value.checked_fetch_sub(tick, AcqRel), Err(Decimal::MIN));
    assert_eq!(value.load(Relaxed), Decimal::MIN);
    assert_eq!(value.fetch_sub(tick, Relaxed), Decimal::MIN);
    assert_eq!(value.load(Relaxed), Decimal::MAX);
    for order in [Relaxed, Acquire, Release, AcqRel, SeqCst] {
        value.store(Decimal::ZERO, Relaxed);
        assert_eq!(value.checked_fetch_sub(tick, order), Ok(Decimal::ZERO));
        assert_eq!(value.load(Relaxed), Decimal::from_raw(-1));
        assert_eq!(
            value.checked_fetch_add(tick, order),
            Ok(Decimal::from_raw(-1))
        );
        assert_eq!(value.load(Relaxed), Decimal::ZERO);
    }
    // Subtraction must not be implemented by negating MIN.
    assert_eq!(
        value.checked_fetch_sub(Decimal::MIN, SeqCst),
        Err(Decimal::ZERO)
    );
    assert_eq!(
        value.checked_fetch_add(Decimal::MIN, SeqCst),
        Ok(Decimal::ZERO)
    );
    assert_eq!(
        value.checked_fetch_sub(Decimal::MIN, SeqCst),
        Ok(Decimal::MIN)
    );
}

#[test]
fn concurrent_add_and_sub_do_not_lose_updates() {
    let value = AtomicDecimal::default();
    let barrier = Barrier::new(8);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..10_000 {
                    value.fetch_add(Decimal::from_raw(3), Relaxed);
                    value.fetch_sub(Decimal::from_raw(1), Relaxed);
                }
            });
        }
    });
    assert_eq!(value.into_inner(), Decimal::from_raw(160_000));
}

#[test]
fn conditional_concurrent_debits_cannot_overdraw() {
    let balance = AtomicDecimal::new(Decimal::from(100));
    let successes = std::sync::atomic::AtomicUsize::new(0);
    let barrier = Barrier::new(8);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..100 {
                    if balance
                        .fetch_update(AcqRel, Acquire, |old| {
                            (old >= Decimal::ONE).then(|| old - Decimal::ONE)
                        })
                        .is_ok()
                    {
                        successes.fetch_add(1, Relaxed);
                    }
                }
            });
        }
    });
    assert_eq!(successes.into_inner(), 100);
    assert_eq!(balance.into_inner(), Decimal::ZERO);
}

#[test]
#[should_panic]
fn invalid_load_order_panics() {
    AtomicDecimal::default().load(Release);
}

#[test]
#[should_panic]
fn invalid_store_order_panics() {
    AtomicDecimal::default().store(Decimal::ZERO, Acquire);
}

#[test]
#[should_panic]
fn invalid_failure_order_panics() {
    let _ = AtomicDecimal::default().compare_exchange(Decimal::ZERO, Decimal::ONE, SeqCst, Release);
}
