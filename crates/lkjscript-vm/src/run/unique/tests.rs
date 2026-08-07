#![allow(clippy::expect_used)]

use super::*;

#[test]
fn stale_owner_and_view_payloads_fail_closed() -> Result<()> {
    let mut runtime = UniqueRuntime::new(&ExecutionConfig::default());
    let owner = runtime.allocate(2)?;
    let view = runtime.borrow(owner, false)?;
    runtime.end_borrow(view)?;
    assert!(runtime.validate_any_view(view).is_err());
    runtime.drop_owner(owner)?;
    assert!(runtime.validate_owner(owner).is_err());
    runtime.verify_empty()?;
    Ok(())
}

#[test]
fn trap_cleanup_releases_owner_and_exclusive_loan_once() -> Result<()> {
    let mut runtime = UniqueRuntime::new(&ExecutionConfig::default());
    let owner = runtime.allocate(4)?;
    let view = runtime.borrow(owner, true)?;
    runtime.set_byte(view, 3, 91)?;
    runtime.cleanup()?;
    assert!(runtime.validate_any_view(view).is_err());
    assert!(runtime.validate_owner(owner).is_err());
    runtime.verify_empty()?;
    Ok(())
}

#[test]
fn byte_vector_crosses_former_per_buffer_limit_under_explicit_heap_policy() -> Result<()> {
    let size = 1_000_001;
    let low = ExecutionConfig {
        max_heap_bytes: size - 1,
        ..ExecutionConfig::default()
    };
    let mut limited = UniqueRuntime::new(&low);
    let error = limited
        .allocate(i64::try_from(size).expect("test size fits i64"))
        .expect_err("explicit low heap policy rejects allocation");
    assert_eq!(
        error.class(),
        lkjscript_core::ErrorClass::Resource(lkjscript_core::ResourceLimitKind::HeapBytes)
    );
    let static_bytes = vec![0x5a; size];
    let error = limited
        .clone_static(&static_bytes)
        .expect_err("explicit low heap policy rejects static clone");
    assert_eq!(
        error.class(),
        lkjscript_core::ErrorClass::Resource(lkjscript_core::ResourceLimitKind::HeapBytes)
    );
    limited.verify_empty()?;

    let high = ExecutionConfig {
        max_heap_bytes: size * 2,
        ..ExecutionConfig::default()
    };
    let mut runtime = UniqueRuntime::new(&high);
    let owner = runtime.allocate(i64::try_from(size).expect("test size fits i64"))?;
    let view = runtime.borrow(owner, false)?;
    assert_eq!(runtime.shared_bytes(view)?.len(), size);
    runtime.end_borrow(view)?;
    runtime.drop_owner(owner)?;
    let static_owner = runtime.clone_static(&static_bytes)?;
    assert_eq!(runtime.bytes_length(static_owner)?, size);
    runtime.drop_owner(static_owner)?;
    runtime.verify_empty()?;
    Ok(())
}

#[test]
fn allocation_failure_publishes_no_owner() -> Result<()> {
    let config = ExecutionConfig {
        max_allocations: 0,
        ..ExecutionConfig::default()
    };
    let mut runtime = UniqueRuntime::new(&config);
    assert!(runtime.allocate(1).is_err());
    runtime.verify_empty()?;
    Ok(())
}
