use super::*;

fn accounting_image(
    installed_source: u64,
    reported_source: u64,
) -> Result<(InstallableImage, FunctionId), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(installed_source),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let source = builder.i64_const(entry, i64::try_from(reported_source)?)?;
    let result = builder.runtime_call(entry, RuntimeCallSlot::EnterFunction, vec![source])?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::new(ImageContracts::current()),
    )?;
    Ok((image, function))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn accounts_sparse_high_source_ids_without_dense_indexing() -> Result<(), Box<dyn std::error::Error>>
{
    let source = 10_000_u64;
    let (image, function) = accounting_image(source, source)?;
    let installed = ExecutableInstaller::default().install(image)?;
    let report =
        installed.invoke_with_config(function, &[], &NativeInvocationConfig::unrestricted())?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(lkjscript_native::NativeValue::Unit)
    );
    assert!(report
        .native_entries()
        .iter()
        .any(|entry| entry.source_function() == source && entry.entries() > 0));
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn rejects_unknown_high_source_entry_accounting() -> Result<(), Box<dyn std::error::Error>> {
    let unknown = 1_u64 << 40;
    let (image, function) = accounting_image(10_000, unknown)?;
    let installed = ExecutableInstaller::default().install(image)?;
    assert_eq!(
        installed.invoke(function, &[]),
        Err(InvocationError::InvalidNativeEntryAccounting(unknown))
    );
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn unrestricted_native_stack_crosses_former_aggregate_ceiling(
) -> Result<(), Box<dyn std::error::Error>> {
    const FUNCTION_COUNT: usize = 70;
    const LOCALS_PER_FUNCTION: usize = 8_000;
    const FORMER_AGGREGATE_CEILING: usize = 4 * 1024 * 1024;

    let mut plan = MachinePlanBuilder::new();
    let signature = Signature::new(vec![ValueType::I64], ValueType::I64)?;
    let mut functions = Vec::new();
    for source in 0..u64::try_from(FUNCTION_COUNT)? {
        functions
            .push(plan.declare_function(SourceFunctionId::new(source + 1_000), signature.clone())?);
    }
    for (index, function) in functions.iter().copied().enumerate() {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        for _ in 0..LOCALS_PER_FUNCTION {
            builder.create_local(ValueType::I64)?;
        }
        let input = builder.parameter(0)?;
        let result = if index == 0 {
            input
        } else {
            builder.call(entry, functions[index - 1], vec![input])?
        };
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }
    let limits = BackendLimits::new(
        128,
        1_024,
        1_000_000,
        LOCALS_PER_FUNCTION,
        32 * 1024 * 1024,
        128 * 1024 * 1024,
        20_000_000,
    );
    let image = encode(
        plan.verify(limits)?,
        EncodingConfig::new(ImageContracts::current()),
    )?;
    let installation_limits = ExecutableLimits::new(
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        1,
    );
    let installed = ExecutableInstaller::new(installation_limits).install(image)?;
    let entry = functions[FUNCTION_COUNT - 1];
    let (value, depth, stack_bytes) = std::thread::Builder::new()
        .name("native-aggregate-stack".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let report = installed
                .invoke_with_config(
                    entry,
                    &[NativeValue::I64(7)],
                    &NativeInvocationConfig::unrestricted(),
                )
                .map_err(|error| error.to_string())?;
            let value = match report.outcome() {
                InvocationOutcome::Returned(NativeValue::I64(value)) => value,
                other => return Err(format!("unexpected aggregate-stack outcome: {other:?}")),
            };
            Ok::<_, String>((
                value,
                report.peak_active_frame_depth(),
                report.peak_native_stack_bytes(),
            ))
        })?
        .join()
        .map_err(|_| "aggregate native stack thread panicked")??;
    assert_eq!(value, 7);
    assert!(depth > 64);
    assert!(stack_bytes > FORMER_AGGREGATE_CEILING, "{stack_bytes}");
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn unrestricted_large_frame_runs_or_reports_actual_thread_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    const LOCAL_COUNT: usize = 132_000;
    const FORMER_FRAME_CEILING: usize = 1024 * 1024;

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(9_000),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    for _ in 0..LOCAL_COUNT {
        builder.create_local(ValueType::I64)?;
    }
    let unit = builder.unit(entry)?;
    builder.return_value(entry, unit)?;
    plan.define_function(builder.finish())?;
    let limits = BackendLimits::new(
        1,
        1,
        LOCAL_COUNT + 1,
        LOCAL_COUNT,
        16 * 1024 * 1024,
        64 * 1024 * 1024,
        5_000_000,
    );
    let image = encode(
        plan.verify(limits)?,
        EncodingConfig::new(ImageContracts::current()),
    )?;
    let installed = std::sync::Arc::new(ExecutableInstaller::default().install(image)?);

    let large_stack = std::sync::Arc::clone(&installed);
    let peak = std::thread::Builder::new()
        .name("native-large-frame".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let report = large_stack
                .invoke_with_config(function, &[], &NativeInvocationConfig::unrestricted())
                .map_err(|error| error.to_string())?;
            if report.outcome() != InvocationOutcome::Returned(NativeValue::Unit) {
                return Err(format!(
                    "unexpected large-frame outcome: {:?}",
                    report.outcome()
                ));
            }
            Ok::<_, String>(report.peak_native_stack_bytes())
        })?
        .join()
        .map_err(|_| "large native frame thread panicked")??;
    assert!(peak > FORMER_FRAME_CEILING, "{peak}");

    let small_stack = std::sync::Arc::clone(&installed);
    let boundary = std::thread::Builder::new()
        .name("native-stack-boundary".into())
        .stack_size(256 * 1024)
        .spawn(move || {
            match small_stack.invoke_with_config(
                function,
                &[],
                &NativeInvocationConfig::unrestricted(),
            ) {
                Err(error) => error,
                Ok(report) => panic!(
                    "actual small thread stack entered the frame: {:?}",
                    report.outcome()
                ),
            }
        })?
        .join()
        .map_err(|_| "small native stack thread panicked")?;
    assert_eq!(
        boundary,
        InvocationError::NativeStackBoundary {
            boundary: lkjscript_executable::NativeStackBoundary::GuardReached,
            retry_safe: true,
        }
    );
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn enforces_contracts_limits_wx_and_repeated_drop() -> Result<(), Box<dyn std::error::Error>> {
    let current = ImageContracts::current();
    let mismatched = ImageContracts::new(
        current.native_layout(),
        current.verified_ssa(),
        current.runtime_calls(),
        current.native_layout(),
    );
    let (image, _) = scalar_image(mismatched)?;
    let installer = ExecutableInstaller::default();
    assert!(matches!(
        installer.install(image),
        Err(InstallError::ContractMismatch { .. })
    ));

    let (image, _) = scalar_image(ImageContracts::current())?;
    let accounting = image.accounting();
    let limits = ExecutableLimits::new(
        accounting.code_bytes() - 1,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
    );
    let installer = ExecutableInstaller::new(limits);
    assert!(matches!(
        installer.install(image),
        Err(InstallError::LimitExceeded(
            ExecutableLimitKind::ObjectCodeBytes
        ))
    ));

    let (first_image, _) = scalar_image(ImageContracts::current())?;
    let (second_image, _) = scalar_image(ImageContracts::current())?;
    let one_object_limits = ExecutableLimits::new(
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        1,
    );
    let one_object_installer = ExecutableInstaller::new(one_object_limits);
    let first_installed = one_object_installer.install(first_image)?;
    assert!(matches!(
        one_object_installer.install(second_image),
        Err(InstallError::LimitExceeded(
            ExecutableLimitKind::ObjectCount
        ))
    ));
    drop(first_installed);
    assert_eq!(one_object_installer.usage().objects(), 0);

    let (image, _) = scalar_image(ImageContracts::current())?;
    let installer = ExecutableInstaller::default();
    {
        let installed = installer.install(image)?;
        assert!(installed.wx_transition_verified());
        let permissions = installed.permissions()?;
        assert!(permissions.readable());
        assert!(!permissions.writable());
        assert!(permissions.executable());
        assert_eq!(installer.usage().objects(), 1);
    }
    assert_eq!(installer.usage().objects(), 0);
    assert_eq!(installer.usage().code_bytes(), 0);

    for _ in 0..32 {
        let (image, entries) = scalar_image(ImageContracts::current())?;
        let installed = installer.install(image)?;
        assert_eq!(
            installed.invoke(entries.direct_call, &[NativeValue::I64(9)])?,
            InvocationOutcome::Returned(NativeValue::I64(18))
        );
        drop(installed);
        assert_eq!(installer.usage().objects(), 0);
    }
    Ok(())
}
