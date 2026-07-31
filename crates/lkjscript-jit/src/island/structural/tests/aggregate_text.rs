use super::support::*;
use lkjscript_core::SemanticPayload;
use lkjscript_native::*;

pub(super) fn option_string() -> Result<(), Box<dyn std::error::Error>> {
    let string = ty(20, StructuralKind::String);
    let option = ty(21, StructuralKind::Enum);
    let aggregate = StructuralAggregateDescriptor::new(
        201,
        option,
        StructuralAggregateKind::Enum(1),
        vec![string],
    );
    let mut plan = MachinePlanBuilder::new();
    let bytes = plan.intern_static_bytes(b"some")?;
    let entry = declare_owner(&mut plan, 12, option)?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let artifact = builder.static_string_const(block, bytes, string)?;
    let string_owner = sc(
        &mut builder,
        block,
        StructuralOperation::PublishStatic {
            value_type: string,
            payload: StructuralPayloadKind::String,
        },
        vec![artifact],
    )?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationCreate(aggregate.clone()),
        vec![],
    )?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationInitialize {
            aggregate: aggregate.clone(),
            field: 0,
        },
        vec![destination, string_owner],
    )?;
    let owner = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationFinish(aggregate),
        vec![destination],
    )?;
    builder.return_value(block, owner)?;
    plan.define_function(builder.finish())?;
    let (_, value, _) = invoke(plan, entry, &[])?;
    let SemanticPayload::Enum {
        tag,
        active_payload,
    } = value
        .ok_or_else(|| std::io::Error::other("option export"))?
        .payload
    else {
        return Err(std::io::Error::other("option payload").into());
    };
    assert_eq!(tag, 1);
    assert_eq!(active_payload[0].utf8(), Some("some"));
    Ok(())
}

pub(super) fn result_path() -> Result<(), Box<dyn std::error::Error>> {
    let path = ty(22, StructuralKind::Path);
    let result = ty(23, StructuralKind::Enum);
    let aggregate = StructuralAggregateDescriptor::new(
        202,
        result,
        StructuralAggregateKind::Enum(2),
        vec![path],
    );
    let mut plan = MachinePlanBuilder::new();
    let bytes = plan.intern_static_bytes(b"/result/path")?;
    let entry = declare_owner(&mut plan, 13, result)?;
    let mut builder = plan.function_builder(entry)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let artifact = builder.static_bytes_const(block, bytes)?;
    let path_owner = sc(
        &mut builder,
        block,
        StructuralOperation::PublishStatic {
            value_type: path,
            payload: StructuralPayloadKind::Path,
        },
        vec![artifact],
    )?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationCreate(aggregate.clone()),
        vec![],
    )?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationInitialize {
            aggregate: aggregate.clone(),
            field: 0,
        },
        vec![destination, path_owner],
    )?;
    let owner = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationFinish(aggregate),
        vec![destination],
    )?;
    builder.return_value(block, owner)?;
    plan.define_function(builder.finish())?;
    let (_, value, _) = invoke(plan, entry, &[])?;
    let SemanticPayload::Enum {
        tag,
        active_payload,
    } = value
        .ok_or_else(|| std::io::Error::other("result export"))?
        .payload
    else {
        return Err(std::io::Error::other("result payload").into());
    };
    assert_eq!(tag, 2);
    assert_eq!(
        active_payload[0].path_bytes(),
        Some(b"/result/path".as_slice())
    );
    Ok(())
}
