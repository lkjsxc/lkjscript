use super::support::*;
use lkjscript_executable::InvocationOutcome;
use lkjscript_native::*;

pub(super) fn path_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let path = ty(11, StructuralKind::Path);
    let mut plan = MachinePlanBuilder::new();
    let bytes = plan.intern_static_bytes(b"/tmp/value")?;
    let entry = plan.declare_function(
        SourceFunctionId::new(11),
        Signature::new(Vec::new(), ValueType::Bool)?,
    )?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let artifact = builder.static_bytes_const(block, bytes)?;
    let left = publish_path(&mut builder, block, path, artifact)?;
    let dynamic_bytes =
        builder.runtime_call(block, RuntimeCallSlot::StaticBytesClone, vec![artifact])?;
    let right = publish_unique_path(&mut builder, block, path, dynamic_bytes)?;
    let view = StructuralViewType::new(103, path, path, false);
    let projection =
        StructuralProjectionDescriptor::new(view, StructuralProjectionKind::Field, Vec::new());
    let left_view = borrow(&mut builder, block, left, &projection)?;
    let right_view = borrow(&mut builder, block, right, &projection)?;
    let equal = sc(
        &mut builder,
        block,
        StructuralOperation::PayloadBytesEqual {
            left: view,
            right: view,
        },
        vec![left_view, right_view],
    )?;
    end_and_drop(&mut builder, block, view, left_view, left)?;
    end_and_drop(&mut builder, block, view, right_view, right)?;
    builder.return_value(block, equal)?;
    plan.define_function(builder.finish())?;
    let (report, _, _) = invoke(plan, entry, &[])?;
    assert!(report.unique_calls() > 0);
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(NativeValue::Bool(true))
    );
    Ok(())
}

fn publish_path(
    builder: &mut FunctionBuilder,
    block: BlockId,
    path: StructuralTypeIdentity,
    artifact: ValueId,
) -> Result<ValueId, PlanError> {
    sc(
        builder,
        block,
        StructuralOperation::PublishStatic {
            value_type: path,
            payload: StructuralPayloadKind::Path,
            storage: StructuralStorageRoute::Unique,
        },
        vec![artifact],
    )
}

fn publish_unique_path(
    builder: &mut FunctionBuilder,
    block: BlockId,
    path: StructuralTypeIdentity,
    owner: ValueId,
) -> Result<ValueId, PlanError> {
    sc(
        builder,
        block,
        StructuralOperation::PublishUnique {
            value_type: path,
            payload: StructuralPayloadKind::Path,
            unique: UniqueType::Bytes,
            storage: StructuralStorageRoute::Unique,
        },
        vec![owner],
    )
}

fn borrow(
    builder: &mut FunctionBuilder,
    block: BlockId,
    owner: ValueId,
    projection: &StructuralProjectionDescriptor,
) -> Result<ValueId, PlanError> {
    sc(
        builder,
        block,
        StructuralOperation::Borrow {
            projection: projection.clone(),
        },
        vec![owner],
    )
}

fn end_and_drop(
    builder: &mut FunctionBuilder,
    block: BlockId,
    view_type: StructuralViewType,
    view: ValueId,
    owner: ValueId,
) -> Result<(), PlanError> {
    sc(
        builder,
        block,
        StructuralOperation::EndView(view_type),
        vec![view],
    )?;
    sc(
        builder,
        block,
        StructuralOperation::Drop(view_type.root()),
        vec![owner],
    )?;
    Ok(())
}
