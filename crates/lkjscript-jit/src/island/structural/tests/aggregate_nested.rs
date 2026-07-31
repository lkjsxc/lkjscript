use super::support::*;
use lkjscript_core::SemanticPayload;
use lkjscript_native::*;

pub(super) fn result_error_tree() -> Result<(), Box<dyn std::error::Error>> {
    let code = ty(24, StructuralKind::I64);
    let error = ty(25, StructuralKind::Product);
    let result = ty(26, StructuralKind::Enum);
    let error_aggregate = StructuralAggregateDescriptor::new(
        203,
        error,
        StructuralAggregateKind::Product,
        vec![code],
    );
    let result_aggregate = StructuralAggregateDescriptor::new(
        204,
        result,
        StructuralAggregateKind::Enum(9),
        vec![error],
    );
    let mut plan = MachinePlanBuilder::new();
    let entry = declare_owner(&mut plan, 14, result)?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    let success = builder.create_block()?;
    let failed = builder.create_block()?;
    builder.set_entry(block)?;
    let raw = builder.i64_const(block, 42)?;
    let error_destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationCreate(error_aggregate.clone()),
        vec![],
    )?;
    let error_destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationInitialize {
            aggregate: error_aggregate.clone(),
            field: 0,
        },
        vec![error_destination, raw],
    )?;
    let error_owner = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationFinish(error_aggregate),
        vec![error_destination],
    )?;
    let result_destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationCreate(result_aggregate.clone()),
        vec![],
    )?;
    let result_destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationInitialize {
            aggregate: result_aggregate.clone(),
            field: 0,
        },
        vec![result_destination, error_owner],
    )?;
    let owner = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationFinish(result_aggregate),
        vec![result_destination],
    )?;
    let enum_view = StructuralViewType::new(205, result, result, false);
    let enum_projection =
        StructuralProjectionDescriptor::new(enum_view, StructuralProjectionKind::Field, vec![]);
    let view = sc(
        &mut builder,
        block,
        StructuralOperation::Borrow {
            projection: enum_projection,
        },
        vec![owner],
    )?;
    let tag = sc(
        &mut builder,
        block,
        StructuralOperation::ObserveTag(enum_view),
        vec![view],
    )?;
    let code_view = StructuralViewType::new(206, result, code, false);
    let code_projection =
        StructuralProjectionDescriptor::new(code_view, StructuralProjectionKind::Field, vec![0, 0]);
    let nested = sc(
        &mut builder,
        block,
        StructuralOperation::Borrow {
            projection: code_projection,
        },
        vec![owner],
    )?;
    let observed = sc(
        &mut builder,
        block,
        StructuralOperation::ObserveI64(code_view),
        vec![nested],
    )?;
    sc(
        &mut builder,
        block,
        StructuralOperation::EndView(enum_view),
        vec![view],
    )?;
    sc(
        &mut builder,
        block,
        StructuralOperation::EndView(code_view),
        vec![nested],
    )?;
    let expected_tag = builder.i64_const(block, 9)?;
    let expected_code = builder.i64_const(block, 42)?;
    let tag_ok = builder.i64_compare(block, I64Comparison::Equal, tag, expected_tag)?;
    let code_ok = builder.i64_compare(block, I64Comparison::Equal, observed, expected_code)?;
    let valid = builder.bool_compare(block, BoolComparison::Equal, tag_ok, code_ok)?;
    builder.branch_if(block, valid, success, failed)?;
    builder.return_value(success, owner)?;
    sc(
        &mut builder,
        failed,
        StructuralOperation::Drop(result),
        vec![owner],
    )?;
    builder.trap(failed, TrapCode::Explicit)?;
    plan.define_function(builder.finish())?;
    let (_, value, _) = invoke(plan, entry, &[])?;
    let SemanticPayload::Enum {
        tag,
        active_payload,
    } = value
        .ok_or_else(|| std::io::Error::other("nested export"))?
        .payload
    else {
        return Err(std::io::Error::other("nested payload").into());
    };
    assert_eq!(tag, 9);
    assert!(matches!(
        active_payload[0].payload,
        SemanticPayload::Product(_)
    ));
    Ok(())
}
