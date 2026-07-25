use super::*;

#[test]
fn rejects_reused_interned_identity_for_distinct_structural_layouts(
) -> Result<(), Box<dyn std::error::Error>> {
    let reused = LayoutIdentity::new(70_000);
    let list = ValueType::Reference(ReferenceType::List(
        reused,
        ValueType::I64.layout_identity(),
    ));
    let option = ValueType::Reference(ReferenceType::Option(
        reused,
        ValueType::I64.layout_identity(),
    ));
    let mut plan = MachinePlanBuilder::new();
    for (source, value_type) in [(1, list), (2, option)] {
        let function = plan.declare_function(
            SourceFunctionId::new(source),
            Signature::new(vec![value_type], value_type)?,
        )?;
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let value = builder.parameter(0)?;
        builder.return_value(entry, value)?;
        plan.define_function(builder.finish())?;
    }
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            "structural layout identity"
        )))
    ));
    Ok(())
}
