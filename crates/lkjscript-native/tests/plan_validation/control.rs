use super::*;

#[test]
fn rejects_missing_terminator_and_uninitialized_local() -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(1),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let _value = builder.i64_const(entry, 1)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(
            VerificationError::MissingTerminator(_)
        ))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(2),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let local = builder.create_local(ValueType::I64)?;
    let value = builder.read_local(entry, local)?;
    builder.return_value(entry, value)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(
            VerificationError::LocalNotInitialized(_)
        ))
    ));
    Ok(())
}

#[test]
fn explicit_trap_uses_bounded_site_identity_without_reference_value(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(3),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    builder.trap_at(entry, TrapCode::Explicit, 7)?;
    plan.define_function(builder.finish())?;
    plan.verify(BackendLimits::default())?;
    Ok(())
}
