use super::super::JitStructuralRuntime;
use super::support::*;
use lkjscript_core::{ExecutionConfig, SemanticPayload};
use lkjscript_executable::{InvocationOutcome, NativeServiceError};
use lkjscript_native::*;

mod sealed;

#[test]
fn conditional_abort_and_unique_backing_transfer_leave_exact_empty_state(
) -> Result<(), Box<dyn std::error::Error>> {
    for abort in [true, false] {
        let (plan, entry) = abort_plan()?;
        let (report, exported, stats) = invoke(plan, entry, &[NativeValue::Bool(abort)])?;
        assert_eq!(
            report.outcome(),
            InvocationOutcome::Returned(NativeValue::Unit)
        );
        assert!(exported.is_none());
        if abort {
            assert_eq!(stats.destinations_aborted, 1);
        } else {
            assert_eq!(stats.destinations_completed, 1);
        }
    }

    let bytes = ty(33, StructuralKind::ByteVector);
    let mut plan = MachinePlanBuilder::new();
    let entry = plan.declare_function(
        SourceFunctionId::new(16),
        Signature::new(Vec::new(), ValueType::StructuralOwner(bytes))?,
    )?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let size = builder.i64_const(block, 3)?;
    let unique = builder.runtime_call(block, RuntimeCallSlot::ByteVectorNew, vec![size])?;
    let owner = sc(
        &mut builder,
        block,
        StructuralOperation::PublishUnique {
            value_type: bytes,
            payload: StructuralPayloadKind::ByteVector,
            unique: UniqueType::ByteVector,
            storage: StructuralStorageRoute::Unique,
        },
        vec![unique],
    )?;
    builder.return_value(block, owner)?;
    plan.define_function(builder.finish())?;
    let (_, exported, stats) = invoke(plan, entry, &[])?;
    let value = exported.ok_or_else(|| std::io::Error::other("byte-vector export"))?;
    assert_eq!(value.payload, SemanticPayload::ByteVector(vec![0, 0, 0]));
    assert!(stats.roots_published > 0);
    Ok(())
}

#[test]
fn stale_owner_and_destination_words_are_stably_trapped() -> Result<(), Box<dyn std::error::Error>>
{
    let value_type = ty(34, StructuralKind::String);
    let mut runtime = JitStructuralRuntime::new(&ExecutionConfig::default())?;
    let owner = runtime
        .publish_static(
            b"stale",
            value_type,
            StructuralPayloadKind::String,
            StructuralStorageRoute::Unique,
        )
        .map_err(|error| std::io::Error::other(format!("publish: {error:?}")))?;
    let moved = runtime
        .move_owner(owner)
        .map_err(|error| std::io::Error::other(format!("move: {error:?}")))?;
    assert_eq!(runtime.drop_owner(owner), Err(NativeServiceError::Trap));
    runtime
        .drop_owner(moved)
        .map_err(|error| std::io::Error::other(format!("drop: {error:?}")))?;
    let product = ty(35, StructuralKind::Product);
    let aggregate = StructuralAggregateDescriptor::new(
        302,
        product,
        StructuralAggregateKind::Product,
        Vec::new(),
    );
    let destination = runtime
        .create_destination(&aggregate, StructuralStorageRoute::Unique)
        .map_err(|error| std::io::Error::other(format!("destination: {error:?}")))?;
    runtime
        .abort_destination(destination)
        .map_err(|error| std::io::Error::other(format!("abort: {error:?}")))?;
    assert_eq!(
        runtime.abort_destination(destination),
        Err(NativeServiceError::Trap)
    );
    let (stats, _) = runtime.finish();
    assert_eq!(stats.teardown_failures, 0);
    Ok(())
}

fn abort_plan() -> Result<(MachinePlanBuilder, FunctionId), Box<dyn std::error::Error>> {
    let storage = StructuralStorageRoute::Unique;
    let scalar = ty(30, StructuralKind::I64);
    let product = ty(31, StructuralKind::Product);
    let aggregate = StructuralAggregateDescriptor::new(
        301,
        product,
        StructuralAggregateKind::Product,
        vec![scalar, scalar],
    );
    let mut plan = MachinePlanBuilder::new();
    let entry = plan.declare_function(
        SourceFunctionId::new(15),
        Signature::new(vec![ValueType::Bool], ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    let abort = builder.create_block()?;
    let finish = builder.create_block()?;
    builder.set_entry(block)?;
    let condition = builder.parameter(0)?;
    let first = builder.i64_const(block, 7)?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationCreate {
            aggregate: aggregate.clone(),
            storage,
        },
        vec![],
    )?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationInitialize {
            aggregate: aggregate.clone(),
            storage,
            field: 0,
        },
        vec![destination, first],
    )?;
    builder.branch_if(block, condition, abort, finish)?;
    sc(
        &mut builder,
        abort,
        StructuralOperation::DestinationAbort {
            aggregate: aggregate.clone(),
            storage,
            initialized: 1,
        },
        vec![destination],
    )?;
    let unit = builder.unit(abort)?;
    builder.return_value(abort, unit)?;
    let second = builder.i64_const(finish, 8)?;
    let destination = sc(
        &mut builder,
        finish,
        StructuralOperation::DestinationInitialize {
            aggregate: aggregate.clone(),
            storage,
            field: 1,
        },
        vec![destination, second],
    )?;
    let owner = sc(
        &mut builder,
        finish,
        StructuralOperation::DestinationFinish { aggregate, storage },
        vec![destination],
    )?;
    sc(
        &mut builder,
        finish,
        StructuralOperation::Drop(product),
        vec![owner],
    )?;
    let unit = builder.unit(finish)?;
    builder.return_value(finish, unit)?;
    plan.define_function(builder.finish())?;
    Ok((plan, entry))
}

fn sc(
    builder: &mut FunctionBuilder,
    block: BlockId,
    operation: StructuralOperation,
    arguments: Vec<ValueId>,
) -> Result<ValueId, PlanError> {
    builder.structural_call(block, StructuralCallDescriptor::new(operation)?, arguments)
}
