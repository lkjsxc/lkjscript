use super::*;

#[test]
fn coordinator_and_application_quotas_form_one_fair_admission_hierarchy(
) -> Result<(), Box<dyn Error>> {
    let identity = CoordinatorIdentity::new(81).ok_or("coordinator")?;
    let system = RuntimeSystem::with_limits(
        identity,
        NonZeroUsize::new(2).ok_or("cache")?,
        RuntimeLimits {
            max_concurrent_invocations: NonZeroUsize::MIN,
            max_total_invocations: NonZeroU64::new(2).ok_or("global total")?,
        },
    );
    let first = system.install(
        manifest(ApplicationKind::Service, 2, 4),
        package(81)?,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let second = system.install(
        manifest(ApplicationKind::Service, 2, 4),
        package(82)?,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let first = system.start(first)?;
    let second = system.start(second)?;
    let outcomes = system.invoke_concurrent(vec![
        InvocationRequest {
            incarnation: first,
            arguments: Vec::new(),
        },
        InvocationRequest {
            incarnation: second,
            arguments: Vec::new(),
        },
    ])?;
    assert!(outcomes.into_iter().all(|outcome| outcome.is_ok()));
    let accounting = system.accounting()?;
    assert_eq!(accounting.total_invocations, 2);
    assert_eq!(accounting.active_invocations, 0);
    assert_eq!(accounting.peak_concurrent, 1);
    assert_eq!(
        system.invoke(first, Vec::new()),
        Err(RuntimeError::QuotaExceeded(
            QuotaKind::CoordinatorTotalInvocations
        ))
    );
    Ok(())
}
