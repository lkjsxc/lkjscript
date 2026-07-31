use super::super::*;
use lkjscript_core::ScopeId;
use lkjscript_executable::{
    ExecutableInstaller, InvocationOutcome, InvocationReport, NativeInvocationConfig,
};
use lkjscript_native::{
    encode, BackendLimits, BlockId, EncodingConfig, FunctionBuilder, FunctionId,
    MachinePlanBuilder, NativeExecutionDomain, NativeValue, PlanError, RuntimeCallSlot, Signature,
    SourceFunctionId, StructuralCallDescriptor, StructuralKind as NativeStructuralKind,
    StructuralOperation, StructuralTypeIdentity, ValueId, ValueType,
};

pub(super) fn ty(id: u64, kind: NativeStructuralKind) -> StructuralTypeIdentity {
    StructuralTypeIdentity::new(id * 2 + 1, id * 2 + 2, kind)
}

pub(super) fn declare_owner(
    plan: &mut MachinePlanBuilder,
    source: u32,
    value_type: StructuralTypeIdentity,
) -> Result<FunctionId, PlanError> {
    plan.declare_function(
        SourceFunctionId::new(source),
        Signature::new(Vec::new(), ValueType::StructuralOwner(value_type))?,
    )
}

pub(super) fn sc(
    builder: &mut FunctionBuilder,
    block: BlockId,
    operation: StructuralOperation,
    arguments: Vec<ValueId>,
) -> Result<ValueId, PlanError> {
    builder.structural_call(block, StructuralCallDescriptor::new(operation)?, arguments)
}

pub(super) fn invoke(
    plan: MachinePlanBuilder,
    entry: FunctionId,
    arguments: &[NativeValue],
) -> Result<
    (
        InvocationReport,
        Option<SemanticValue>,
        NativeStructuralStats,
    ),
    Box<dyn std::error::Error>,
> {
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    assert_eq!(
        image.execution_domain(),
        NativeExecutionDomain::CollectorFree
    );
    assert!(!image.bytes().is_empty());
    assert!(!image.entries().is_empty());
    assert!(!image.structural_runtime_sites().is_empty());
    assert!(image.safepoints().is_empty());
    assert!(image.heap_runtime_sites().is_empty());
    assert!(image
        .frames()
        .iter()
        .flat_map(|frame| frame.homes())
        .all(|home| !matches!(home.value_type(), ValueType::Reference(_))));
    assert!(image
        .runtime_calls()
        .contains(&RuntimeCallSlot::StructuralDispatch));
    assert!(!image.runtime_calls().iter().any(|slot| {
        matches!(
            slot,
            RuntimeCallSlot::CollectReference
                | RuntimeCallSlot::HeapDispatch
                | RuntimeCallSlot::PublishSafepoint
        )
    }));
    for entry in image.entries() {
        if matches!(entry.signature().result(), ValueType::StructuralOwner(_)) {
            let frame = image
                .frames()
                .iter()
                .find(|frame| frame.function() == entry.function())
                .ok_or_else(|| std::io::Error::other("missing structural frame"))?;
            assert!(!frame.returned_structural_owners().is_empty());
        }
    }
    assert!(image
        .frames()
        .iter()
        .flat_map(|frame| frame.homes())
        .any(|home| {
            matches!(
                home.value_type(),
                ValueType::StructuralOwner(_)
                    | ValueType::StructuralView(_)
                    | ValueType::StructuralDestination(_)
            )
        }));
    let installed = ExecutableInstaller::default().install(image)?;
    assert!(installed.wx_transition_verified());
    let scope = ScopeId::new(1).ok_or_else(|| std::io::Error::other("scope"))?;
    let mut services = JitIslandServices::new(scope, &ExecutionConfig::default())?;
    let report = installed.invoke_island_with_services(
        entry,
        arguments,
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    let exported = match report.outcome() {
        InvocationOutcome::Returned(NativeValue::StructuralOwner(owner)) => Some(
            services
                .export_structural(owner)
                .map_err(|error| std::io::Error::other(format!("export: {error:?}")))?,
        ),
        _ => None,
    };
    let (_, unique, structural, _, _, empty) = services.finish();
    assert!(empty);
    assert_eq!(structural.live_roots, 0);
    assert_eq!(structural.live_loans, 0);
    assert_eq!(structural.live_views, 0);
    assert_eq!(structural.live_destinations, 0);
    assert_eq!(structural.release_backlog, 0);
    assert_eq!(structural.teardown_failures, 0);
    assert_eq!(unique.live_owners, 0);
    assert_eq!(unique.live_loans, 0);
    assert!(!report.collector_runtime());
    assert_eq!(report.collection_calls(), 0);
    assert_eq!(report.maximum_roots(), 0);
    assert!(report.exact_root_counts().is_empty());
    assert_eq!(report.heap_operation_attempts(), 0);
    assert_eq!(report.heap_operation_successes(), 0);
    assert_eq!(report.barrier_count(), 0);
    assert!(report.cleanup_failures().is_empty());
    assert_eq!(report.omitted_cleanup_failures(), 0);
    assert_eq!(report.active_frame_depth(), 0);
    assert_eq!(report.active_value_homes(), 0);
    assert!(
        report
            .native_entries()
            .iter()
            .map(|entry| entry.entries())
            .sum::<u64>()
            > 0
    );
    assert!(report.structural_calls() > 0);
    Ok((report, exported, structural))
}
