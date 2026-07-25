use super::*;

#[test]
fn heap_dispatch_sites_verify_arbitrary_frame_home_arguments_and_classes(
) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = ValueType::Reference(ReferenceType::Buf);
    assert_eq!(
        HeapCallDescriptor::new(
            HeapOperation::BufSet,
            vec![buffer, ValueType::I64],
            ValueType::Unit,
            AllocationClass::None,
            StoreClass::Scalar,
        ),
        Err(PlanError::InvalidHeapCall)
    );
    assert_eq!(
        HeapCallDescriptor::new(
            HeapOperation::BufSet,
            vec![buffer, ValueType::I64, ValueType::I64],
            ValueType::Unit,
            AllocationClass::Bounded,
            StoreClass::Scalar,
        ),
        Err(PlanError::InvalidHeapCall)
    );

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(77),
        Signature::new(vec![buffer], ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let index = builder.i64_const(entry, 0)?;
    let byte = builder.i64_const(entry, 255)?;
    let descriptor = HeapCallDescriptor::new(
        HeapOperation::BufSet,
        vec![buffer, ValueType::I64, ValueType::I64],
        ValueType::Unit,
        AllocationClass::None,
        StoreClass::Scalar,
    )?;
    let result = builder.heap_call(entry, descriptor, vec![input, index, byte])?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    assert_eq!(image.heap_runtime_sites().len(), 1);
    assert_eq!(image.heap_runtime_sites()[0].arguments().len(), 3);
    assert_eq!(
        image.heap_runtime_sites()[0].result().value_type(),
        ValueType::Unit
    );
    assert!(image
        .runtime_calls()
        .contains(&RuntimeCallSlot::HeapDispatchV1));
    assert_eq!(image.safepoints()[0].stack_map().roots().len(), 1);
    Ok(())
}
