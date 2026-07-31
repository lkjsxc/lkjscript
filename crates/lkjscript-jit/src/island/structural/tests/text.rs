use super::{support::*, text_path::path_fixture};
use lkjscript_native::*;

#[test]
fn generated_string_and_path_payload_operations_are_collector_free(
) -> Result<(), Box<dyn std::error::Error>> {
    let string = ty(10, StructuralKind::String);
    let mut plan = MachinePlanBuilder::new();
    let text = plan.intern_static_bytes(b"dynamic-text")?;
    let entry = plan.declare_function(
        SourceFunctionId::new(10),
        Signature::new(Vec::new(), ValueType::StructuralOwner(string))?,
    )?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    let utf8_ok = builder.create_block()?;
    let dynamic_ok = builder.create_block()?;
    let returned = builder.create_block()?;
    let failed = builder.create_block()?;
    builder.set_entry(block)?;
    let artifact = builder.static_string_const(block, text, string)?;
    let static_bytes = builder.static_bytes_const(block, text)?;
    let owner = call(
        &mut builder,
        block,
        StructuralOperation::PublishStatic {
            value_type: string,
            payload: StructuralPayloadKind::String,
        },
        vec![artifact],
    )?;
    let owner = call(
        &mut builder,
        block,
        StructuralOperation::Move(string),
        vec![owner],
    )?;
    let dynamic_bytes =
        builder.runtime_call(block, RuntimeCallSlot::StaticBytesClone, vec![static_bytes])?;
    let dynamic_owner = call(
        &mut builder,
        block,
        StructuralOperation::PublishUnique {
            value_type: string,
            payload: StructuralPayloadKind::String,
            unique: UniqueType::Bytes,
        },
        vec![dynamic_bytes],
    )?;
    let field = projection(101, string, StructuralProjectionKind::Field);
    let utf8 = projection(102, string, StructuralProjectionKind::Utf8);
    let left = call(
        &mut builder,
        block,
        StructuralOperation::Borrow {
            projection: field.clone(),
        },
        vec![owner],
    )?;
    let zero = builder.i64_const(block, 0)?;
    let end = builder.i64_const(block, 12)?;
    let right = call(
        &mut builder,
        block,
        StructuralOperation::Borrow {
            projection: utf8.clone(),
        },
        vec![owner, zero, end],
    )?;
    let equal = call(
        &mut builder,
        block,
        StructuralOperation::PayloadBytesEqual {
            left: field.view_type(),
            right: utf8.view_type(),
        },
        vec![left, right],
    )?;
    let valid = call(
        &mut builder,
        block,
        StructuralOperation::PayloadUtf8Valid(utf8.view_type()),
        vec![right],
    )?;
    let dynamic = projection(105, string, StructuralProjectionKind::Field);
    let dynamic_view = call(
        &mut builder,
        block,
        StructuralOperation::Borrow {
            projection: dynamic.clone(),
        },
        vec![dynamic_owner],
    )?;
    let dynamic_equal = call(
        &mut builder,
        block,
        StructuralOperation::PayloadBytesEqual {
            left: field.view_type(),
            right: dynamic.view_type(),
        },
        vec![left, dynamic_view],
    )?;
    call(
        &mut builder,
        block,
        StructuralOperation::EndView(field.view_type()),
        vec![left],
    )?;
    call(
        &mut builder,
        block,
        StructuralOperation::EndView(utf8.view_type()),
        vec![right],
    )?;
    call(
        &mut builder,
        block,
        StructuralOperation::EndView(dynamic.view_type()),
        vec![dynamic_view],
    )?;
    call(
        &mut builder,
        block,
        StructuralOperation::Drop(string),
        vec![dynamic_owner],
    )?;
    let exclusive_type = StructuralViewType::new(104, string, string, true);
    let exclusive = StructuralProjectionDescriptor::new(
        exclusive_type,
        StructuralProjectionKind::Field,
        Vec::new(),
    );
    let exclusive = call(
        &mut builder,
        block,
        StructuralOperation::Borrow {
            projection: exclusive,
        },
        vec![owner],
    )?;
    call(
        &mut builder,
        block,
        StructuralOperation::PayloadUtf8Valid(exclusive_type),
        vec![exclusive],
    )?;
    call(
        &mut builder,
        block,
        StructuralOperation::EndView(exclusive_type),
        vec![exclusive],
    )?;
    builder.branch_if(block, equal, utf8_ok, failed)?;
    builder.branch_if(utf8_ok, valid, dynamic_ok, failed)?;
    builder.branch_if(dynamic_ok, dynamic_equal, returned, failed)?;
    builder.return_value(returned, owner)?;
    call(
        &mut builder,
        failed,
        StructuralOperation::Drop(string),
        vec![owner],
    )?;
    builder.trap(failed, TrapCode::Explicit)?;
    plan.define_function(builder.finish())?;
    let (report, exported, stats) = invoke(plan, entry, &[])?;
    assert!(report.unique_calls() > 0);
    assert_eq!(
        exported
            .and_then(|value| value.utf8().map(str::to_owned))
            .as_deref(),
        Some("dynamic-text")
    );
    assert!(stats.roots_moved > 0);

    path_fixture()?;
    Ok(())
}

fn call(
    builder: &mut FunctionBuilder,
    block: BlockId,
    operation: StructuralOperation,
    arguments: Vec<ValueId>,
) -> Result<ValueId, PlanError> {
    builder.structural_call(block, StructuralCallDescriptor::new(operation)?, arguments)
}

fn projection(
    id: u64,
    value_type: StructuralTypeIdentity,
    kind: StructuralProjectionKind,
) -> StructuralProjectionDescriptor {
    StructuralProjectionDescriptor::new(
        StructuralViewType::new(id, value_type, value_type, false),
        kind,
        Vec::new(),
    )
}
