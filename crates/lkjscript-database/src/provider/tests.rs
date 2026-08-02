use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_host::HostError;

use super::fresh_provider_identity;

#[test]
fn provider_identity_exhaustion_is_terminal_without_wraparound() {
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(fresh_provider_identity(&counter), Ok(u64::MAX - 1));
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);

    for _ in 0..2 {
        assert!(matches!(
            fresh_provider_identity(&counter),
            Err(HostError::Io { operation, message })
                if operation == "attach database tenant"
                    && message == "provider identity exhausted"
        ));
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    }
}

#[test]
fn provider_identity_zero_state_fails_closed() {
    let counter = AtomicU64::new(0);
    assert!(fresh_provider_identity(&counter).is_err());
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}
