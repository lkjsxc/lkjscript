use crate::{
    encode, AllocationClass, BackendLimits, EncodingConfig, HeapCallDescriptor, HeapOperation,
    MachinePlanBuilder, ReferenceType, RuntimeCallSlot, Signature, SourceFunctionId, StoreClass,
    ValueType,
};

#[test]
fn integrity_rejects_malformed_heap_site_home_and_safepoint(
) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(9),
        Signature::new(vec![buffer], ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let index = builder.i64_const(entry, 0)?;
    let byte = builder.i64_const(entry, 1)?;
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
    let mut image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    let original = image.heap_runtime_sites[0].result.rbp_displacement;
    image.heap_runtime_sites[0].result.rbp_displacement = -8;
    assert_eq!(
        image.validate_integrity(),
        Err(super::ImageIntegrityError::HeapRuntimeSite)
    );
    image.heap_runtime_sites[0].result.rbp_displacement = original;
    image.heap_runtime_sites[0].safepoint = u32::MAX;
    assert_eq!(
        image.validate_integrity(),
        Err(super::ImageIntegrityError::HeapRuntimeSite)
    );
    Ok(())
}

#[test]
fn integrity_rejects_out_of_frame_root_without_accounting_change(
) -> Result<(), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let function =
        plan.declare_function(SourceFunctionId::new(1), Signature::new(vec![buf], buf)?)?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let collected =
        builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![input])?;
    builder.return_value(entry, collected)?;
    plan.define_function(builder.finish())?;
    let mut image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    image.accounting.metadata_bytes -= 1;
    assert_eq!(
        image.validate_integrity(),
        Err(super::ImageIntegrityError::MetadataAccountingMismatch)
    );
    image.accounting.metadata_bytes += 1;
    image.safepoints[0].id = 7;
    assert_eq!(
        image.validate_integrity(),
        Err(super::ImageIntegrityError::Safepoint)
    );
    image.safepoints[0].id = 0;
    image.safepoints[0].stack_map.roots[0].rbp_displacement = -8;
    assert_eq!(
        image.validate_integrity(),
        Err(super::ImageIntegrityError::Safepoint)
    );
    image.safepoints[0].stack_map.roots[0].rbp_displacement =
        image.root_requirements[0].roots[0].rbp_displacement;
    let _omitted_live_root = image.safepoints[0].stack_map.roots.pop();
    image.accounting.metadata_bytes -= 16;
    assert_eq!(
        image.validate_integrity(),
        Err(super::ImageIntegrityError::RootRequirement)
    );
    Ok(())
}
