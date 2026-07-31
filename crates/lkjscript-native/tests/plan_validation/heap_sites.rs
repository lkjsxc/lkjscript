use super::*;

#[test]
fn runtime_value_sites_verify_arbitrary_frame_home_arguments_and_classes(
) -> Result<(), Box<dyn std::error::Error>> {
    let product = ValueType::Reference(ReferenceType::RegionProduct(
        LayoutIdentity::product(0),
        [7; 32],
    ));
    let operation = || HeapOperation::WithProductField {
        product: 0,
        field: 0,
        field_type: ValueType::I64,
    };
    assert_eq!(
        HeapCallDescriptor::new(
            operation(),
            vec![product],
            product,
            AllocationClass::Bounded,
            StoreClass::Initialization,
        ),
        Err(PlanError::InvalidHeapCall)
    );
    assert_eq!(
        HeapCallDescriptor::new(
            operation(),
            vec![product, ValueType::I64],
            product,
            AllocationClass::None,
            StoreClass::Initialization,
        ),
        Err(PlanError::InvalidHeapCall)
    );

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(77),
        Signature::new(vec![product, ValueType::I64], product)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let owner = builder.parameter(0)?;
    let replacement = builder.parameter(1)?;
    let descriptor = HeapCallDescriptor::new(
        operation(),
        vec![product, ValueType::I64],
        product,
        AllocationClass::Bounded,
        StoreClass::Initialization,
    )?;
    let result = builder.heap_call(entry, descriptor, vec![owner, replacement])?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    assert_eq!(image.heap_runtime_sites().len(), 1);
    assert_eq!(image.heap_runtime_sites()[0].arguments().len(), 2);
    assert_eq!(image.heap_runtime_sites()[0].result().value_type(), product);
    assert!(image
        .runtime_calls()
        .contains(&RuntimeCallSlot::HeapDispatch));
    Ok(())
}
