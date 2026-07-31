use super::*;

#[test]
fn verifies_exact_reference_signatures_and_type_use() -> Result<(), Box<dyn std::error::Error>> {
    let product_ref = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(0)));
    let other_product = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(1)));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(10),
        Signature::new(vec![product_ref], product_ref)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let one = builder.i64_const(entry, 1)?;
    let invalid = builder.i64_add(entry, input, one)?;
    builder.return_value(entry, invalid)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            _
        )))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(11),
        Signature::new(vec![other_product], product_ref)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    builder.return_value(entry, input)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::InvalidReturn(
            _
        )))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(12),
        Signature::new(vec![ValueType::I64], product_ref)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let integer = builder.parameter(0)?;
    let invalid = builder.runtime_call(entry, RuntimeCallSlot::CollectReference, vec![integer])?;
    builder.return_value(entry, invalid)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            _
        )))
    ));
    Ok(())
}
