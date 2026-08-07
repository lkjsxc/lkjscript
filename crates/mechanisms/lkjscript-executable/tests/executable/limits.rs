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
fn grows_active_frame_tracking_beyond_sixty_four() -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let signature = Signature::new(vec![ValueType::I64], ValueType::I64)?;
    let mut functions = Vec::new();
    for source in 0..100_u64 {
        functions
            .push(plan.declare_function(SourceFunctionId::new(source + 1_000), signature.clone())?);
    }
    for (index, function) in functions.iter().copied().enumerate() {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
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
        16_384,
        1_024,
        4 * 1024 * 1024,
        4 * 1024 * 1024,
        100_000,
    );
    let image = encode(
        plan.verify(limits)?,
        EncodingConfig::new(ImageContracts::current()),
    )?;
    let installed = ExecutableInstaller::default().install(image)?;
    let report = installed.invoke_with_config(
        functions[99],
        &[NativeValue::I64(7)],
        &NativeInvocationConfig::unrestricted(),
    )?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(NativeValue::I64(7))
    );
    assert!(report.peak_active_frame_depth() > 64);
    assert_eq!(report.active_frame_depth(), 0);
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
