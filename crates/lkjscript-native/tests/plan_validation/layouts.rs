use super::*;

#[test]
fn rejects_reused_interned_identity_for_distinct_structural_layouts(
) -> Result<(), Box<dyn std::error::Error>> {
    let reused = LayoutIdentity::new(70_000);
    let list = ValueType::Reference(ReferenceType::List(
        reused,
        900,
        ValueType::I64.layout_identity(),
        901,
    ));
    let list_f64 = ValueType::Reference(ReferenceType::List(
        reused,
        900,
        ValueType::F64.layout_identity(),
        902,
    ));
    let mut plan = MachinePlanBuilder::new();
    for (source, value_type) in [(1, list), (2, list_f64)] {
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

#[test]
fn rejects_reused_region_layout_for_distinct_product_contracts(
) -> Result<(), Box<dyn std::error::Error>> {
    let layout = LayoutIdentity::product(7);
    let first = ValueType::Reference(ReferenceType::RegionProduct(layout, [1; 32]));
    let second = ValueType::Reference(ReferenceType::RegionProduct(layout, [2; 32]));
    let mut plan = MachinePlanBuilder::new();
    for (source, value_type) in [(1, first), (2, second)] {
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
