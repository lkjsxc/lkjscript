#![allow(clippy::panic)]

use lkjscript_native::{
    encode, AbiVersions, BackendLimits, EncodingConfig, FunctionId, InstallableImage,
    MachinePlanBuilder, NativeReference, NativeValue, ReferenceType, RuntimeCallSlot,
    RuntimeOutcome, Signature, SourceFunctionId, TrapCode, ValueType,
};
use lkjscript_sys::executable::{
    ExecutableInstaller, InvocationOutcome, NativeInvocationConfig, NativeResourceLimitKind,
    NativeRoot, NativeRuntimeServices, NativeServiceError,
};

#[derive(Clone, Copy)]
struct ReferenceEntries {
    exact_local: FunctionId,
    caller: FunctionId,
    trap_caller: FunctionId,
    exit_caller: FunctionId,
    deadline_caller: FunctionId,
    resource_caller: FunctionId,
    host_caller: FunctionId,
}

fn reference_image() -> Result<(InstallableImage, ReferenceEntries), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let exact_local = plan.declare_function(
        SourceFunctionId::new(1),
        Signature::new(vec![buf, buf], buf)?,
    )?;
    let callee =
        plan.declare_function(SourceFunctionId::new(2), Signature::new(vec![buf], buf)?)?;
    let caller =
        plan.declare_function(SourceFunctionId::new(3), Signature::new(vec![buf], buf)?)?;
    let trap = plan.declare_function(
        SourceFunctionId::new(4),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let exit = plan.declare_function(
        SourceFunctionId::new(5),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let deadline = plan.declare_function(
        SourceFunctionId::new(6),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let resource = plan.declare_function(
        SourceFunctionId::new(7),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let host = plan.declare_function(
        SourceFunctionId::new(8),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let trap_caller = plan.declare_function(
        SourceFunctionId::new(9),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let exit_caller = plan.declare_function(
        SourceFunctionId::new(10),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let deadline_caller = plan.declare_function(
        SourceFunctionId::new(11),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let resource_caller = plan.declare_function(
        SourceFunctionId::new(12),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let host_caller = plan.declare_function(
        SourceFunctionId::new(13),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;

    {
        let mut builder = plan.function_builder(exact_local)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let live = builder.parameter(0)?;
        let _dead = builder.parameter(1)?;
        let local = builder.create_local(buf)?;
        let _write = builder.write_local(entry, local, live)?;
        let _collected =
            builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![live])?;
        let returned = builder.read_local(entry, local)?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(callee)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let collected =
            builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![input])?;
        builder.return_value(entry, collected)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(caller)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let returned = builder.call(entry, callee, vec![input])?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(trap)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        builder.trap(entry, TrapCode::Explicit)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(exit)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let status = builder.i64_const(entry, 23)?;
        builder.exit(entry, status)?;
        plan.define_function(builder.finish())?;
    }
    for (function, outcome) in [
        (deadline, RuntimeOutcome::DeadlineExceeded),
        (resource, RuntimeOutcome::ResourceLimitExceeded),
        (host, RuntimeOutcome::HostFailure),
    ] {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        builder.outcome(entry, outcome)?;
        plan.define_function(builder.finish())?;
    }
    for (function, callee) in [
        (trap_caller, trap),
        (exit_caller, exit),
        (deadline_caller, deadline),
        (resource_caller, resource),
        (host_caller, host),
    ] {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let returned = builder.call(entry, callee, Vec::new())?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }

    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::new(AbiVersions::current()),
    )?;
    Ok((
        image,
        ReferenceEntries {
            exact_local,
            caller,
            trap_caller,
            exit_caller,
            deadline_caller,
            resource_caller,
            host_caller,
        },
    ))
}

#[derive(Default)]
struct RecordingServices {
    observed: Vec<Vec<(ReferenceType, u64)>>,
    replacement: Option<u64>,
    failure: Option<NativeServiceError>,
}

impl NativeRuntimeServices for RecordingServices {
    fn collect_references(&mut self, roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        self.observed.push(
            roots
                .iter()
                .map(|root| (root.reference_type(), root.opaque_word()))
                .collect(),
        );
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        if let Some(replacement) = self.replacement {
            for root in roots {
                root.set_opaque_word(replacement);
            }
        }
        Ok(())
    }
}

fn buf(word: u64) -> NativeValue {
    NativeValue::Reference(NativeReference::buf(word))
}

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
        &[buf(11), buf(22)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(report.outcome(), InvocationOutcome::Returned(buf(33)));
    assert_eq!(report.collection_calls(), 1);
    assert_eq!(report.exact_root_counts(), &[2]);
    assert_eq!(report.maximum_roots(), 2);
    assert_eq!(report.peak_active_frame_depth(), 1);
    assert_eq!(report.active_frame_depth(), 0);
    assert_eq!(services.observed.len(), 1);
    assert_eq!(services.observed[0].len(), 2);
    assert!(services.observed[0]
        .iter()
        .all(|item| *item == (ReferenceType::Buf, 11)));
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
        &[buf(44)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(report.outcome(), InvocationOutcome::Returned(buf(44)));
    assert_eq!(report.peak_active_frame_depth(), 2);
    assert_eq!(report.active_frame_depth(), 0);
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
        &[buf(1)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(failed.outcome(), InvocationOutcome::HostFailure);
    assert_eq!(failed.peak_active_frame_depth(), 2);
    assert_eq!(failed.active_frame_depth(), 0);

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
