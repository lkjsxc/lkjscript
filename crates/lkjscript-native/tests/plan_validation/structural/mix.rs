use super::*;

#[test]
fn structural_and_owned_resource_mix_fails_before_image_installation(
) -> Result<(), Box<dyn std::error::Error>> {
    let value_type = ty(5, StructuralKind::String);
    let mut plan = MachinePlanBuilder::new();
    let bytes = plan.intern_static_bytes(b"mix")?;
    let function = plan.declare_function(
        SourceFunctionId::new(7),
        Signature::new(
            vec![ValueType::Resource(ResourceKind::FileReader)],
            ValueType::Unit,
        )?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let artifact = builder.static_string_const(block, bytes, value_type)?;
    let owner = sc(
        &mut builder,
        block,
        StructuralOperation::PublishStatic {
            value_type,
            payload: StructuralPayloadKind::String,
            storage: StructuralStorageRoute::Unique,
        },
        vec![artifact],
    )?;
    sc(
        &mut builder,
        block,
        StructuralOperation::Drop(value_type),
        vec![owner],
    )?;
    let unit = builder.unit(block)?;
    builder.return_value(block, unit)?;
    plan.define_function(builder.finish())?;
    let verified = plan.verify(BackendLimits::default())?;
    assert!(matches!(encode(verified), Err(NativeError::Image(_))));
    Ok(())
}
