use super::*;
use lkjscript_native::RuntimeOutcome;

fn outcome_image(
    outcome: RuntimeOutcome,
) -> Result<(InstallableImage, FunctionId), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(50_000 + outcome as u64),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    builder.outcome(entry, outcome)?;
    plan.define_function(builder.finish())?;
    let image = encode(plan.verify(BackendLimits::default())?)?;
    Ok((image, function))
}

fn prepare_error(
    installed: &InstalledImage,
    entry: FunctionId,
    arguments: &[NativeValue],
    config: &NativeInvocationConfig,
) -> PreEntryError {
    let mut services = NoopNativeIslandRuntimeServices;
    match installed.prepare_invocation(entry, arguments, config, &mut services) {
        Ok(prepared) => {
            drop(prepared);
            panic!("invocation unexpectedly prepared")
        }
        Err(error) => error,
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn preparation_rejects_bad_arguments_and_pre_entry_faults_without_entry(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = scalar_image()?;
    let installed = ExecutableInstaller::default().install(image)?;

    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::I64(1)],
            &NativeInvocationConfig::unrestricted(),
        ),
        PreEntryError::ArgumentCount {
            expected: 2,
            actual: 1,
        }
    );
    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::Bool(true), NativeValue::I64(1)],
            &NativeInvocationConfig::unrestricted(),
        ),
        PreEntryError::ArgumentType {
            index: 0,
            expected: Box::new(ValueType::I64),
            actual: Box::new(ValueType::Bool),
        }
    );
    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::I64(1), NativeValue::I64(2)],
            &NativeInvocationConfig::unrestricted().with_max_active_frames(usize::MAX),
        ),
        PreEntryError::BookkeepingAllocationFailed
    );
    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::I64(1), NativeValue::I64(2)],
            &NativeInvocationConfig::unrestricted().with_native_stack_requirement(usize::MAX),
        ),
        PreEntryError::NativeStackUnavailable(
            lkjscript_executable::NativeStackError::FrameArithmeticOverflow,
        )
    );
    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::I64(1), NativeValue::I64(2)],
            &NativeInvocationConfig::unrestricted().with_cancellation_requested(true),
        ),
        PreEntryError::Cancelled
    );
    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::I64(1), NativeValue::I64(2)],
            &NativeInvocationConfig::limited(1, Some(std::time::Duration::from_nanos(1)),),
        ),
        PreEntryError::DeadlineExceeded
    );
    assert_eq!(
        prepare_error(
            &installed,
            entries.checked_add,
            &[NativeValue::I64(1), NativeValue::I64(2)],
            &NativeInvocationConfig::limited(0, None),
        ),
        PreEntryError::ResourceLimitExceeded(NativeResourceLimitKind::PollFuel)
    );
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn prepared_drop_is_non_entering_and_enter_consumes_exactly_once(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = scalar_image()?;
    let installer = ExecutableInstaller::default();
    let installed = installer.install(image)?;
    let mut services = NoopNativeIslandRuntimeServices;
    let prepared = installed.prepare_invocation(
        entries.direct_call,
        &[NativeValue::I64(21)],
        &NativeInvocationConfig::unrestricted(),
        &mut services,
    )?;
    drop(prepared);
    assert_eq!(installer.usage().objects(), 1);
    assert!(installed.permissions()?.executable());

    let mut services = NoopNativeIslandRuntimeServices;
    let prepared = installed.prepare_invocation(
        entries.direct_call,
        &[NativeValue::I64(21)],
        &NativeInvocationConfig::unrestricted(),
        &mut services,
    )?;
    let report = prepared.enter()?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(NativeValue::I64(42))
    );
    assert_eq!(
        report
            .native_entries()
            .iter()
            .map(|count| count.entries())
            .sum::<u64>(),
        2
    );

    drop(installed);
    assert_eq!(installer.usage().objects(), 0);
    assert_eq!(installer.usage().code_bytes(), 0);
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn trap_resource_deadline_and_host_are_entered_outcomes() -> Result<(), Box<dyn std::error::Error>>
{
    let (image, entries) = scalar_image()?;
    let installed = ExecutableInstaller::default().install(image)?;
    assert_eq!(
        installed.invoke(
            entries.checked_add,
            &[NativeValue::I64(i64::MAX), NativeValue::I64(1)],
        )?,
        InvocationOutcome::Trapped(TrapCode::I64Overflow)
    );

    for (native, expected) in [
        (
            RuntimeOutcome::ResourceLimitExceeded,
            InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::PollFuel),
        ),
        (
            RuntimeOutcome::DeadlineExceeded,
            InvocationOutcome::DeadlineExceeded,
        ),
        (RuntimeOutcome::HostFailure, InvocationOutcome::HostFailure),
    ] {
        let (image, entry) = outcome_image(native)?;
        let installed = ExecutableInstaller::default().install(image)?;
        let mut services = NoopNativeIslandRuntimeServices;
        let prepared = installed.prepare_invocation(
            entry,
            &[],
            &NativeInvocationConfig::unrestricted(),
            &mut services,
        )?;
        assert_eq!(prepared.enter()?.outcome(), expected);
    }
    Ok(())
}
