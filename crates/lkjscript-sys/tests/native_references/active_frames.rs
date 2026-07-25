use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn active_frame_bound_uses_unregistered_epilogue_and_reference_images_repeat(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = reference_image()?;
    let installer = ExecutableInstaller::default();
    let installed = installer.install(image)?;
    let config = NativeInvocationConfig::default().with_max_active_frames(1);
    let report = installed.invoke_with_config(entries.caller, &[buf(9)], &config)?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::ActiveFrames)
    );
    assert_eq!(report.peak_active_frame_depth(), 1);
    assert_eq!(report.active_frame_depth(), 0);
    drop(installed);
    assert_eq!(installer.usage().objects(), 0);

    for _ in 0..8 {
        let (image, entries) = reference_image()?;
        let installed = installer.install(image)?;
        let report = installed.invoke_with_config(
            entries.exact_local,
            &[buf(7), buf(8)],
            &NativeInvocationConfig::default(),
        )?;
        assert_eq!(report.outcome(), InvocationOutcome::Returned(buf(7)));
        assert_eq!(report.active_frame_depth(), 0);
        drop(installed);
        assert_eq!(installer.usage().objects(), 0);
    }
    Ok(())
}

fn large_frame_image() -> Result<(InstallableImage, FunctionId), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(40),
        Signature::new(Vec::new(), ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let returned = builder.i64_const(entry, 7)?;
    for value in 1..10_000 {
        let _unused = builder.i64_const(entry, i64::from(value))?;
    }
    builder.return_value(entry, returned)?;
    plan.define_function(builder.finish())?;
    let limits = BackendLimits::new(1, 1, 12_000, 0, 1024 * 1024, 1024 * 1024, 200_000);
    Ok((
        encode(plan.verify(limits)?, EncodingConfig::default())?,
        function,
    ))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn large_frame_reservation_fails_safely_on_small_thread_stack_and_zero_frame_cap(
) -> Result<(), Box<dyn std::error::Error>> {
    let handle =
        std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(|| -> Result<(), String> {
                let (image, entry) = large_frame_image().map_err(|error| error.to_string())?;
                let installed = ExecutableInstaller::default()
                    .install(image)
                    .map_err(|error| error.to_string())?;
                let stack_limited = installed
                    .invoke_with_config(
                        entry,
                        &[],
                        &NativeInvocationConfig::default()
                            .with_native_stack_limits(usize::MAX, usize::MAX),
                    )
                    .map_err(|error| error.to_string())?;
                assert_eq!(
                    stack_limited.outcome(),
                    InvocationOutcome::ResourceLimitExceeded(
                        NativeResourceLimitKind::NativeStackBytes
                    )
                );
                assert_eq!(stack_limited.peak_native_stack_bytes(), 0);
                assert_eq!(stack_limited.reserved_native_stack_bytes(), 0);

                for config in [
                    NativeInvocationConfig::default().with_native_stack_limits(0, usize::MAX),
                    NativeInvocationConfig::default().with_native_stack_limits(usize::MAX, 0),
                ] {
                    let budget_limited = installed
                        .invoke_with_config(entry, &[], &config)
                        .map_err(|error| error.to_string())?;
                    assert_eq!(
                        budget_limited.outcome(),
                        InvocationOutcome::ResourceLimitExceeded(
                            NativeResourceLimitKind::NativeStackBytes
                        )
                    );
                    assert_eq!(budget_limited.reserved_native_stack_bytes(), 0);
                }

                let frame_limited = installed
                    .invoke_with_config(
                        entry,
                        &[],
                        &NativeInvocationConfig::default()
                            .with_max_active_frames(0)
                            .with_native_stack_limits(usize::MAX, usize::MAX),
                    )
                    .map_err(|error| error.to_string())?;
                assert_eq!(
                    frame_limited.outcome(),
                    InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::ActiveFrames)
                );
                assert_eq!(frame_limited.peak_active_frame_depth(), 0);
                assert_eq!(frame_limited.reserved_native_stack_bytes(), 0);
                Ok(())
            })?;
    let result = handle
        .join()
        .map_err(|_| std::io::Error::other("small-stack native thread panicked"))?;
    result.map_err(std::io::Error::other)?;
    Ok(())
}
