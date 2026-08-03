use super::support::*;
use lkjscript_native::*;

#[test]
fn generated_inferred_call_like_shared_borrow_returns_exact_owner(
) -> Result<(), Box<dyn std::error::Error>> {
    let string = ty(40, StructuralKind::String);
    let mut plan = MachinePlanBuilder::new();
    let bytes = plan.intern_static_bytes(b"callee-borrow")?;
    let callee = plan.declare_function(
        SourceFunctionId::new(17),
        Signature::new(
            vec![ValueType::StructuralOwner(string)],
            ValueType::StructuralOwner(string),
        )?,
    )?;
    let root = plan.declare_function(
        SourceFunctionId::new(18),
        Signature::new(Vec::new(), ValueType::StructuralOwner(string))?,
    )?;
    let mut callee_builder = plan.function_builder(callee)?;
    let block = callee_builder.create_block()?;
    callee_builder.set_entry(block)?;
    let owner = callee_builder.parameter(0)?;
    let view_type = StructuralViewType::new(401, string, string, false);
    let projection =
        StructuralProjectionDescriptor::new(view_type, StructuralProjectionKind::Field, Vec::new());
    let view = sc(
        &mut callee_builder,
        block,
        StructuralOperation::Borrow { projection },
        vec![owner],
    )?;
    sc(
        &mut callee_builder,
        block,
        StructuralOperation::PayloadUtf8Valid(view_type),
        vec![view],
    )?;
    sc(
        &mut callee_builder,
        block,
        StructuralOperation::EndView(view_type),
        vec![view],
    )?;
    callee_builder.return_value(block, owner)?;
    plan.define_function(callee_builder.finish())?;

    let mut root_builder = plan.function_builder(root)?;
    let block = root_builder.create_block()?;
    root_builder.set_entry(block)?;
    let artifact = root_builder.static_string_const(block, bytes, string)?;
    let owner = sc(
        &mut root_builder,
        block,
        StructuralOperation::PublishStatic {
            value_type: string,
            payload: StructuralPayloadKind::String,
            storage: StructuralStorageRoute::Unique,
        },
        vec![artifact],
    )?;
    let returned = root_builder.call(block, callee, vec![owner])?;
    root_builder.return_value(block, returned)?;
    plan.define_function(root_builder.finish())?;
    let (_, value, stats) = invoke(plan, root, &[])?;
    assert_eq!(
        value
            .and_then(|value| value.utf8().map(str::to_owned))
            .as_deref(),
        Some("callee-borrow")
    );
    assert!(stats.loans_started > 0);
    assert_eq!(stats.loans_started, stats.loans_ended);
    Ok(())
}

fn sc(
    builder: &mut FunctionBuilder,
    block: BlockId,
    operation: StructuralOperation,
    arguments: Vec<ValueId>,
) -> Result<ValueId, PlanError> {
    builder.structural_call(block, StructuralCallDescriptor::new(operation)?, arguments)
}
