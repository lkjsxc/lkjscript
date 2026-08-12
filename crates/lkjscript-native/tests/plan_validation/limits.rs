use super::*;

#[test]
fn rejects_type_mismatch_unsupported_signature_and_code_limit(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(3),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let boolean = builder.bool_const(entry, true)?;
    let integer = builder.i64_const(entry, 1)?;
    let invalid = builder.i64_add(entry, boolean, integer)?;
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
        SourceFunctionId::new(4),
        Signature::new(
            vec![
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
            ],
            ValueType::I64,
        )?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let value = builder.parameter(0)?;
    builder.return_value(entry, value)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(
            VerificationError::UnsupportedSignature(_)
        ))
    ));

    let limits = BackendLimits::new(4, 8, 64, 8, 16, 4_096, 1_000);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(5),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let value = builder.i64_const(entry, 42)?;
    builder.return_value(entry, value)?;
    plan.define_function(builder.finish())?;
    let verified = plan.verify(limits)?;
    assert!(matches!(encode(verified), Err(NativeError::Encode(_))));
    Ok(())
}
