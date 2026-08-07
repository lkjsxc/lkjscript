#![allow(clippy::expect_used)]

use super::*;

#[test]
fn stale_forged_and_wrong_kind_words_fail_closed_without_leaks() {
    let mut runtime = JitUniqueRuntime::new(&ExecutionConfig::default()).expect("unique runtime");
    let owner = runtime.allocate(2).expect("owner");
    let shared = runtime
        .borrow(owner, LoanType::ByteSlice)
        .expect("shared loan");
    let forged_kind = NativeLoan::byte_slice_mut(shared.opaque_word());
    assert_eq!(runtime.length(forged_kind), Err(NativeServiceError::Trap));
    assert_eq!(
        runtime.borrow(owner, LoanType::ByteSliceMut),
        Err(NativeServiceError::Trap)
    );
    runtime.end_borrow(shared).expect("end shared loan");
    runtime.drop_owner(owner).expect("drop exact owner");
    assert_eq!(runtime.move_owner(owner), Err(NativeServiceError::Trap));
    assert_eq!(
        runtime.move_owner(NativeUnique::byte_vector(1)),
        Err(NativeServiceError::Trap)
    );
    let stats = runtime.finish();
    assert!(stats.stale_or_forged_failures >= 4);
    assert_eq!(stats.drops, 1);
    assert_eq!(
        (
            stats.live_owners,
            stats.live_loans,
            stats.release_backlog,
            stats.teardown_failures,
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn bytes_layout_transfers_and_forged_words_fail_closed() {
    let mut runtime = JitUniqueRuntime::new(&ExecutionConfig::default()).expect("unique runtime");
    let vector = runtime.allocate(3).expect("vector owner");
    let bytes = runtime.freeze(vector).expect("freeze backing");
    assert_eq!(runtime.move_owner(vector), Err(NativeServiceError::Trap));
    runtime.move_bytes(bytes).expect("exact bytes owner");
    assert_eq!(
        runtime.move_bytes(NativeUnique::bytes(u64::MAX)),
        Err(NativeServiceError::Trap)
    );
    let vector = runtime.thaw(bytes).expect("thaw backing");
    assert_eq!(runtime.move_bytes(bytes), Err(NativeServiceError::Trap));
    runtime.drop_owner(vector).expect("drop transferred owner");
    let stats = runtime.finish();
    assert!(stats.stale_or_forged_failures >= 3);
    assert_eq!(
        (
            stats.live_owners,
            stats.live_loans,
            stats.release_backlog,
            stats.teardown_failures,
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn native_byte_vector_uses_explicit_heap_policy_beyond_former_buffer_limit() {
    let size = 1_000_001_usize;
    let low = ExecutionConfig {
        max_heap_bytes: size - 1,
        ..ExecutionConfig::default()
    };
    let mut limited = JitUniqueRuntime::new(&low).expect("limited unique runtime");
    assert_eq!(
        limited.allocate(i64::try_from(size).expect("test size fits i64")),
        Err(NativeServiceError::ResourceLimitExceeded)
    );
    assert_eq!(limited.last_resource(), Some(ResourceLimitKind::HeapBytes));
    let static_bytes = vec![0x5a; size];
    assert_eq!(
        limited.clone_static_bytes(&static_bytes),
        Err(NativeServiceError::ResourceLimitExceeded)
    );
    assert_eq!(limited.last_resource(), Some(ResourceLimitKind::HeapBytes));
    assert_eq!(limited.finish().live_owners, 0);

    let high = ExecutionConfig {
        max_heap_bytes: size * 2,
        ..ExecutionConfig::default()
    };
    let mut runtime = JitUniqueRuntime::new(&high).expect("high-policy unique runtime");
    let owner = runtime
        .allocate(i64::try_from(size).expect("test size fits i64"))
        .expect("large byte vector");
    let view = runtime
        .borrow(owner, LoanType::ByteSlice)
        .expect("borrow large vector");
    assert_eq!(
        runtime.length(view),
        Ok(i64::try_from(size).expect("test size fits i64"))
    );
    runtime.end_borrow(view).expect("end large borrow");
    runtime.drop_owner(owner).expect("drop large owner");
    let bytes = runtime
        .clone_static_bytes(&static_bytes)
        .expect("large static bytes clone");
    let loan = runtime
        .borrow(bytes, LoanType::Bytes)
        .expect("borrow large bytes");
    assert_eq!(
        runtime.bytes_length(loan),
        Ok(i64::try_from(size).expect("test size fits i64"))
    );
    runtime.end_borrow(loan).expect("end large bytes borrow");
    runtime.drop_owner(bytes).expect("drop large bytes");
    assert_eq!(runtime.finish().live_owners, 0);
}

#[test]
fn configured_allocation_limit_is_structured_and_atomic() {
    let config = ExecutionConfig {
        max_allocations: 0,
        ..ExecutionConfig::default()
    };
    let mut runtime = JitUniqueRuntime::new(&config).expect("limited unique runtime");
    assert_eq!(
        runtime.allocate(1),
        Err(NativeServiceError::ResourceLimitExceeded)
    );
    assert_eq!(
        runtime.clone_static_bytes(&[1]),
        Err(NativeServiceError::ResourceLimitExceeded)
    );
    assert_eq!(
        runtime.last_resource(),
        Some(ResourceLimitKind::Allocations)
    );
    let stats = runtime.finish();
    assert_eq!((stats.live_owners, stats.live_loans), (0, 0));
}
