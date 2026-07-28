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
