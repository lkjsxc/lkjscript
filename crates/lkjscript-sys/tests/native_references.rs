#![allow(clippy::panic)]

use lkjscript_native::{
    encode, AbiVersions, AllocationClass, BackendLimits, EncodingConfig, FunctionId,
    HeapCallDescriptor, HeapOperation, HeapRuntimeSite, InstallableImage, LayoutIdentity,
    MachinePlanBuilder, NativeReference, NativeValue, ReferenceType, RuntimeCallSlot,
    RuntimeOutcome, Signature, SourceFunctionId, StoreClass, TrapCode, ValueType,
};
use lkjscript_sys::executable::{
    ExecutableInstaller, ExecutableLimits, InvocationOutcome, NativeInvocationConfig,
    NativeResourceLimitKind, NativeRoot, NativeRuntimeServices, NativeServiceError,
};

#[derive(Clone, Copy)]
enum HeapFailure {
    Trap,
    Resource,
    Host,
}

struct FailingHeapService {
    failure: HeapFailure,
    calls: usize,
}

impl NativeRuntimeServices for FailingHeapService {
    fn collect_references(&mut self, _roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        Ok(())
    }

    fn heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        assert_eq!(arguments.len(), 3);
        assert_eq!(site.arguments().len(), 3);
        self.calls += 1;
        Err(match self.failure {
            HeapFailure::Trap => NativeServiceError::Trap,
            HeapFailure::Resource => NativeServiceError::ResourceLimitExceeded,
            HeapFailure::Host => NativeServiceError::HostFailure,
        })
    }
}

#[test]
fn generic_heap_dispatch_propagates_service_status_and_unwinds(
) -> Result<(), Box<dyn std::error::Error>> {
    let product = ValueType::Reference(ReferenceType::Product(LayoutIdentity::new(1)));
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(42),
        Signature::new(Vec::new(), product)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let values = [
        builder.i64_const(entry, 1)?,
        builder.i64_const(entry, 2)?,
        builder.i64_const(entry, 3)?,
    ];
    let descriptor = HeapCallDescriptor::new(
        HeapOperation::ProductValue {
            product: 0,
            fields: 3,
        },
        vec![ValueType::I64; 3],
        product,
        AllocationClass::Bounded,
        StoreClass::Initialization,
    )?;
    let result = builder.heap_call(entry, descriptor, values.to_vec())?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    let installed = ExecutableInstaller::default().install(image)?;
    for (failure, expected) in [
        (
            HeapFailure::Trap,
            InvocationOutcome::Trapped(TrapCode::Explicit),
        ),
        (
            HeapFailure::Resource,
            InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::RuntimeService),
        ),
        (HeapFailure::Host, InvocationOutcome::HostFailure),
    ] {
        let mut service = FailingHeapService { failure, calls: 0 };
        let report = installed.invoke_with_services(
            function,
            &[],
            &NativeInvocationConfig::default(),
            &mut service,
        )?;
        assert_eq!(report.outcome(), expected);
        assert_eq!(report.active_frame_depth(), 0);
        assert_eq!(report.reserved_native_stack_bytes(), 0);
        assert_eq!(service.calls, 1);
    }
    Ok(())
}

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
    assert!(report.peak_native_stack_bytes() > 0);
    assert_eq!(report.reserved_native_stack_bytes(), 0);
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

    services.failure = Some(NativeServiceError::ResourceLimitExceeded);
    let service_limited = installed.invoke_with_services(
        entries.caller,
        &[buf(1)],
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

fn shallow_wide_root_image() -> Result<(InstallableImage, FunctionId), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let sink = plan.declare_function(
        SourceFunctionId::new(41),
        Signature::new(vec![buf, buf], ValueType::Unit)?,
    )?;
    let wide = plan.declare_function(SourceFunctionId::new(42), Signature::new(vec![buf], buf)?)?;

    let mut sink_builder = plan.function_builder(sink)?;
    let sink_entry = sink_builder.create_block()?;
    sink_builder.set_entry(sink_entry)?;
    let unit = sink_builder.unit(sink_entry)?;
    sink_builder.return_value(sink_entry, unit)?;
    plan.define_function(sink_builder.finish())?;

    let mut builder = plan.function_builder(wide)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let mut locals = Vec::new();
    for _ in 0..1024 {
        let local = builder.create_local(buf)?;
        let _write = builder.write_local(entry, local, input)?;
        locals.push(local);
    }
    let collected =
        builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![input])?;
    for pair in locals.chunks_exact(2) {
        let first = builder.read_local(entry, pair[0])?;
        let second = builder.read_local(entry, pair[1])?;
        let _call = builder.call(entry, sink, vec![first, second])?;
    }
    builder.return_value(entry, collected)?;
    plan.define_function(builder.finish())?;
    let limits = BackendLimits::new(
        2,
        2,
        4_096,
        1_024,
        4 * 1024 * 1024,
        32 * 1024 * 1024,
        2_000_000,
    );
    Ok((
        encode(plan.verify(limits)?, EncodingConfig::default())?,
        wide,
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn shallow_1025_root_map_reserves_dynamically_under_aggregate_cap(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entry) = shallow_wide_root_image()?;
    assert_eq!(
        image
            .safepoints()
            .iter()
            .map(|safepoint| safepoint.stack_map().roots().len())
            .max(),
        Some(1025)
    );
    let unlimited_install = ExecutableLimits::new(
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
    );
    let installed = ExecutableInstaller::new(unlimited_install).install(image)?;
    let mut services = RecordingServices::default();
    let report = installed.invoke_with_services(
        entry,
        &[buf(55)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(report.outcome(), InvocationOutcome::Returned(buf(55)));
    assert_eq!(report.exact_root_counts(), &[1025]);
    assert_eq!(report.maximum_roots(), 1025);
    assert_eq!(report.reserved_native_stack_bytes(), 0);
    assert_eq!(services.observed.len(), 1);
    assert_eq!(services.observed[0].len(), 1025);
    Ok(())
}
