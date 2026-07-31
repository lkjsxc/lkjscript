use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn generated_collection_materializes_only_live_exact_roots(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = reference_image()?;
    let installed = ExecutableInstaller::default().install(image)?;
    let mut services = RecordingServices {
        replacement: Some(33),
        ..RecordingServices::default()
    };
    let report = installed.invoke_with_services(
        entries.exact_local,
        &[product_ref(11), product_ref(22)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(product_ref(33))
    );
    assert_eq!(report.collection_calls(), 1);
    assert_eq!(report.exact_root_counts(), &[2]);
    assert_eq!(report.maximum_roots(), 2);
    assert_eq!(report.peak_active_frame_depth(), 1);
    assert_eq!(report.active_frame_depth(), 0);
    assert!(report.peak_native_stack_bytes() > 0);
    assert_eq!(report.reserved_native_stack_bytes(), 0);
    assert_eq!(services.observed.len(), 1);
    assert_eq!(services.observed[0].len(), 2);
    assert!(services.observed[0]
        .iter()
        .all(|item| { *item == (ReferenceType::Product(LayoutIdentity::product(0)), 11,) }));
    assert!(!services.observed[0].iter().any(|item| item.1 == 22));
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn collecting_callee_exposes_caller_and_callee_chain() -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = reference_image()?;
    let installed = ExecutableInstaller::default().install(image)?;
    let mut services = RecordingServices::default();
    let report = installed.invoke_with_services(
        entries.caller,
        &[product_ref(44)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(product_ref(44))
    );
    assert_eq!(report.peak_active_frame_depth(), 2);
    assert_eq!(report.active_frame_depth(), 0);
    assert!(report.peak_active_value_homes() > 0);
    assert_eq!(report.active_value_homes(), 0);
    assert_eq!(report.exact_root_counts(), &[2]);
    assert_eq!(services.observed[0].len(), 2);
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn service_failure_and_all_structured_paths_unregister_frames(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = reference_image()?;
    let installed = ExecutableInstaller::default().install(image)?;
    let mut services = RecordingServices {
        failure: Some(NativeServiceError::HostFailure),
        ..RecordingServices::default()
    };
    let failed = installed.invoke_with_services(
        entries.caller,
        &[product_ref(1)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(failed.outcome(), InvocationOutcome::HostFailure);
    assert_eq!(failed.peak_active_frame_depth(), 2);
    assert_eq!(failed.active_frame_depth(), 0);

    services.failure = Some(NativeServiceError::ResourceLimitExceeded);
    let service_limited = installed.invoke_with_services(
        entries.caller,
        &[product_ref(1)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(
        service_limited.outcome(),
        InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::RuntimeService)
    );
    assert_ne!(
        service_limited.outcome(),
        InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::MaterializedRoots)
    );
    assert_eq!(service_limited.active_frame_depth(), 0);

    let expected = [
        (
            entries.trap_caller,
            InvocationOutcome::Trapped(TrapCode::Explicit),
        ),
        (entries.exit_caller, InvocationOutcome::Exited(23)),
        (entries.deadline_caller, InvocationOutcome::DeadlineExceeded),
        (
            entries.resource_caller,
            InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::PollFuel),
        ),
        (entries.host_caller, InvocationOutcome::HostFailure),
    ];
    for (entry, outcome) in expected {
        let report =
            installed.invoke_with_config(entry, &[], &NativeInvocationConfig::default())?;
        assert_eq!(report.outcome(), outcome);
        assert_eq!(report.active_frame_depth(), 0);
        assert_eq!(report.peak_active_frame_depth(), 2);
    }
    Ok(())
}
